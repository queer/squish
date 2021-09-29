use std::error::Error;
use std::fs::{self, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::process;

use libsquish::squishfile::{LayerSpec, Squishfile};
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot, close, dup, dup2};

/// Sets up and runs the given container. This function will
/// - Create the necessary paths for the container to exist
/// - Redirect this process' stdout/stderr to log files
/// - Bind-mount the container's rootfs
/// - Bind-mount in the special devices
///   - `/dev/null`
///   - `/dev/zero`
///   - `/dev/random`
///   - `/dev/urandom`
/// - Mount all non-app layers
/// - Bind-mount the app layer into `/app`
/// - Run the container
pub fn setup_and_run_container(
    rootfs: &String,
    path: &String,
    _container_id: &String,
    squishfile: &Squishfile,
) -> Result<(), Box<dyn Error>> {
    let container_path = format!("{}/rootfs", &path);
    fs::create_dir_all(&container_path).expect("couldn't create rootfs directory!");

    // redirect stdout/err
    let stdout_dup = dup(1)?;
    let stderr_dup = dup(2)?;
    close(1)?;
    close(2)?;

    let stdout_log = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("{}/output.log", &path))?;
    let stdout_log_fd = stdout_log.into_raw_fd();
    let stderr_log = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("{}/error.log", &path))?;
    let stderr_log_fd = stderr_log.into_raw_fd();

    dup2(stdout_log_fd, stdout_dup)?;
    dup2(stderr_log_fd, stderr_dup)?;
    close(stdout_dup)?;
    close(stderr_dup)?;

    // Bindmount rootfs ro
    bind_mount(
        &rootfs,
        &container_path,
        MsFlags::MS_RDONLY | MsFlags::MS_NOATIME | MsFlags::MS_NOSUID,
    )?;

    // Bind-mount *nix stuff in
    println!(">> bindmounting devices");
    bind_mount_dev("/dev/null", &format!("{}/dev/null", container_path))?;
    bind_mount_dev("/dev/zero", &format!("{}/dev/zero", container_path))?;
    bind_mount_dev("/dev/random", &format!("{}/dev/random", container_path))?;
    bind_mount_dev("/dev/urandom", &format!("{}/dev/urandom", container_path))?;
    println!(">> bindmounting devices finished!");

    // Bindmount /tmp rw
    let tmp_path = format!("{}/tmp", &path);
    fs::create_dir_all(&tmp_path)?;
    bind_mount(&tmp_path, &format!("{}/tmp", container_path), MsFlags::MS_NOSUID)?;

    for (layer_name, layer) in squishfile.layers() {
        if layer_name != "alpine" && layer_name != "app" {
            bind_mount_layer::<&str>(&container_path, layer_name, layer, None)?;
        } else if layer_name == "app" {
            bind_mount_layer(&container_path, layer_name, layer, Some("/app/"))?;
        }
    }

    // chroot!
    chroot(container_path.as_str()).expect("couldn't chroot!?");
    chdir("/").expect("couldn't chdir to /!?");

    // TODO: Should totally be blocking on slirp4netns being up here...

    run_in_container(&squishfile);
    println!(">> done!");
    Ok(())
}

fn bind_mount_layer<TO>(
    container_path: &String,
    layer_name: &String,
    layer: &LayerSpec,
    target_override: Option<TO>,
) -> Result<(), Box<dyn Error>>
where
    TO: Into<String>,
{
    // Bind-mount squishfile layers
    println!(">> bindmounting {:?} => {:?}", layer.path(), layer.target());
    if layer.path().is_none() && layer.version().is_none() {
        panic!("squishfile: nothing to mount for layer {}!?", layer_name);
    }
    let target = match layer.target() {
        Some(target) => target.clone(),
        None => {
            if layer.path().is_some() {
                // If path but no target, mount into /app
                let target = layer
                    .path()
                    .as_ref()
                    .unwrap()
                    .replace("../", "")
                    .replace("./", "");
                format!("/app/{}", target)
            } else if layer.version().is_none() {
                // If no path and no target and no version, panic
                panic!("squishfile no path or version for layer {}", layer_name);
            } else {
                // If no path or target, but there is a version, mount into /sdk
                format!("/sdk/{}", layer_name)
            }
        }
    };
    let target = if let Some(target_override) = target_override {
        let file_name = Path::new(&target)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        format!("{}/{}", target_override.into(), file_name)
    } else {
        target
    };
    let target = format!("{}/{}", container_path, target);
    // if layer.path().is_none() && layer.version().is_some() && layer.target().is_none() {
    //     todo!("mounting squish layer normally");
    // }
    let path = layer.path().as_ref().unwrap();
    let mount_path = Path::new(path);
    // Yeah this is technically racy, but literally who cares?
    if mount_path.exists() {
        let meta = fs::metadata(path)?;
        if meta.is_dir() {
            touch_dir(&target)?;
        } else if meta.is_file() {
            let target_path = Path::new(&target);
            // TODO: Do this better
            let parent = target_path.parent().unwrap().to_str().unwrap().to_string();
            touch_dir(&parent)?;
            touch(&target)?;
        } else {
            println!(">> mount is not a directory or file");
        }
        bind_mount(
            path,
            &target,
            MsFlags::MS_RDONLY | MsFlags::MS_NOATIME | MsFlags::MS_NOSUID,
        )?;
    } else {
        println!(">> mount didn't exist");
    }
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

fn touch(path: &String) -> Result<(), Box<dyn Error>> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn touch_dir(path: &String) -> Result<(), Box<dyn Error>> {
    match fs::create_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn run_in_container(squishfile: &Squishfile) {
    println!(">> inside the container!");
    println!(">> i am {}", process::id());
    println!(
        ">> running: {} {:?}",
        squishfile.run().command(),
        squishfile.run().args()
    );

    std::process::Command::new(squishfile.run().command())
        .args(squishfile.run().args())
        .output()
        .unwrap();
}
