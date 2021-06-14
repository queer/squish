use std::error::Error;
use std::fs;
use std::os::unix::io::IntoRawFd;
use std::process;

use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot, close, dup, dup2};

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
    std::process::Command::new("sleep").arg("30").output().unwrap();
}