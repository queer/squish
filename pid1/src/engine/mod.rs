use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::os::unix::io::IntoRawFd;
use std::process;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use futures::TryStreamExt;
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot, close, dup, dup2};
use rand::Rng;
use rtnetlink::Handle;

pub fn setup_container(rootfs: &String, container_id: &String) -> Result<(), Box<dyn Error>> {
    // TODO: lol error checking
    let container_path = format!("container/{}/rootfs", &container_id);
    fs::create_dir_all(&container_path).expect("couldn't create rootfs folder!");

    // redirect stdout/err
    let stdout_dup = dup(1).unwrap();
    let stderr_dup = dup(2).unwrap();
    close(1).unwrap();
    close(2).unwrap();

    let stdout_log = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("container/{}/output.log", &container_id))
        .unwrap();
    let stdout_log_fd = stdout_log.into_raw_fd();
    let stderr_log = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("container/{}/error.log", &container_id))
        .unwrap();
    let stderr_log_fd = stderr_log.into_raw_fd();

    // TODO: Lol buffering
    dup2(stdout_log_fd, stdout_dup).unwrap();
    dup2(stderr_log_fd, stderr_dup).unwrap();
    close(stdout_dup).unwrap();
    close(stderr_dup).unwrap();

    // Bindmount rootfs ro
    bind_mount(&rootfs, &container_path, MsFlags::MS_RDONLY)?;

    // Bind-mount *nix stuff in
    println!(">> bindmounting devices");
    bind_mount_dev("/dev/null", &format!("{}/dev/null", container_path))?;
    bind_mount_dev("/dev/zero", &format!("{}/dev/zero", container_path))?;
    bind_mount_dev("/dev/random", &format!("{}/dev/random", container_path))?;
    bind_mount_dev("/dev/urandom", &format!("{}/dev/urandom", container_path))?;
    println!(">> bindmounting devices finished!");

    // Networking
    // println!(">> binding network interfaces");
    // let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?; // TODO: lol
    // runtime.block_on(setup_network_in_new_netns(container_id))?;
    // println!(">> network ifaces bound!");

    // TODO: User-defined bindmounts

    // chroot!
    chroot(container_path.as_str()).expect("couldn't chroot!?");
    chdir("/").expect("couldn't chdir to /!?");

    // TODO: Should totally be blocking on slirp4netns being up here...

    run_in_container();
    println!(">> done!");
    Ok(())
}

fn bind_mount_dev(dev: &'static str, target: &String) -> Result<(), Box<dyn Error>> {
    println!(">> bindmount dev {} -> {}", dev, target);
    mount(
        Some(dev),
        target.as_str(),
        Some(""),
        MsFlags::MS_BIND,
        Some(""),
    )?;
    Ok(())
}

fn bind_mount(src: &String, target: &String, flags: MsFlags) -> Result<(), Box<dyn Error>> {
    println!(">> bindmount {} -> {}", src, target);
    mount(
        Some(src.as_str()),
        target.as_str(),
        Some(""),
        MsFlags::MS_BIND | flags,
        Some(""),
    )?;
    Ok(())
}

async fn setup_network_in_new_netns(container_id: &String) -> Result<String, Box<dyn Error>> {
    let (_connection, handle, _receiver) = rtnetlink::new_connection().unwrap();
    let mut rng = rand::thread_rng();

    // Add input veth that peers with output veth
    println!(">> creating input and output veths");
    let input_name = format!("squishvethin@{}", container_id);
    let output_name = format!("squishvethout@{}", container_id);
    handle
        .link()
        .add()
        .veth(input_name.clone(), output_name.clone())
        .execute()
        .await?;
    let input_veth = locate_device(&handle, &input_name).await?;
    let output_veth = locate_device(&handle, &output_name).await?;
    println!(">> input={}, output={}", input_veth, output_veth);

    // Make sure veth applies to this namespace
    println!(">> setting input veth to this netns");
    handle
        .link()
        .set(input_veth)
        .setns_by_pid(process::id())
        .execute()
        .await?;

    // Assign a random IP in the range 169.254.1.0-169.254.254.255, for
    // compliance with RFC3927
    // TODO: Handle conflicts
    let first = rng.gen_range(1..254);
    let second = rng.gen_range(0..255);
    let new_ip = format!("169.254.{}.{}", first, second);
    println!(">> binding to {}", new_ip);
    let addr = IpAddr::V4(Ipv4Addr::new(169, 254, first, second));
    handle
        .address()
        .add(input_veth, addr, 255)
        .execute()
        .await?;

    // Create bridge if not exists
    // TODO: Better error-checking here
    println!(">> setting up bridge");
    match handle
        .link()
        .add()
        .bridge("squish-bridge".into())
        .execute()
        .await
    {
        Err(e) => eprintln!(
            "couldn't setup bridge, most likely because it exists: {}",
            e
        ),
        _ => {}
    };
    let bridge_veth = locate_device(&handle, "squish-bridge").await?;
    println!(">> bridge={}", bridge_veth);

    // Peer output to the bridge
    println!(">> peer output -> bridge");
    handle
        .link()
        .set(output_veth)
        .master(bridge_veth)
        .execute()
        .await?;

    // Set bridge up
    println!(">> up bridge");
    handle.link().set(bridge_veth).up().execute().await?;

    // Set output veth up
    println!(">> up output veth");
    handle.link().set(output_veth).up().execute().await?;

    // Set input veth up
    println!(">> up input veth");
    handle.link().set(input_veth).up().execute().await?;

    // Set lo up (current netns only)
    println!(">> up current ns lo");
    let lo_index = locate_device(&handle, "lo").await?;
    handle.link().set(lo_index).up().execute().await?;

    println!(">> add bridge addr {}", addr);
    // Add IP for bridge
    match handle.address().add(bridge_veth, addr, 255).execute().await {
        Ok(_) => {}
        Err(e) => eprintln!(
            "couldn't setup bridge ip, most likely because it exists: {}",
            e
        ),
    };

    // Route default by bridge
    println!("route bridge via gateway");
    handle
        .route()
        .add()
        .v4()
        .gateway(Ipv4Addr::new(10, 69, 69, 69))
        .execute()
        .await?;

    // Set nftables masquerade on this ip range
    // TODO: How?

    // sysctl -w net.ipv4.ip_forward=1
    // TODO: How?
    Ok(new_ip)
}

fn err<T: Into<Box<dyn Error + Send + Sync>>>(reason: T) -> Box<dyn Error> {
    Box::new(std::io::Error::new(ErrorKind::Other, reason))
}

async fn locate_device<T: Into<String> + Clone + Display>(
    handle: &Handle,
    name: T,
) -> Result<u32, Box<dyn Error>> {
    if let Some(link) = handle
        .link()
        .get()
        .set_name_filter(name.clone().into())
        .execute()
        .try_next()
        .await?
    {
        Ok(link.header.index)
    } else {
        Err(err(format!("unable to locate netlink idx for '{}'", name)))
    }
}

fn run_in_container() {
    println!(">> inside the container!");
    println!(">> i am {}", process::id());

    if let Ok(paths) = fs::read_dir("/") {
        println!(">> my rootfs has:");
        for path in paths {
            println!(">>    {}", path.unwrap().path().display());
        }
    } else {
        println!(">> warning: could not read_dir /");
    }
    println!(">> running nc on port 2000");
    Command::new("/usr/bin/ncat")
        .args(vec!["-l", "2000", "--keep-open", "--exec", "'/bin/cat'"])
        .output()
        .unwrap();
    println!(">> nc done!");
}
