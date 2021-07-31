use std::error::Error;
use std::fs::{self, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::process;

use libsquish::squishfile::Squishfile;
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot, close, dup, dup2};

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
    // TODO: Determine rootfs from squishfile versions
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

    // TODO: User-defined bindmounts
    // TODO: Bindmount SDK layers

    // Bindmount app
    let app = squishfile
        .layers()
        .get("app")
        .expect("squishfile has no app layer!?");
    let app_file_name = Path::new(app.path().as_ref().unwrap())
        .file_name()
        .expect("squishfile app has no filename!?")
        .to_str()
        .expect("Couldn't convert filename to string!?")
        .to_string();
    // TODO: This should handle automatic extraction of tarballs / zips
    println!(">> bindmounting app");
    let app_bind_path = &format!("{}/app/{}", container_path, app_file_name);
    touch(&app_bind_path)?;
    bind_mount(
        app.path().as_ref().unwrap(),
        app_bind_path,
        MsFlags::MS_RDONLY | MsFlags::MS_NOATIME | MsFlags::MS_NOSUID,
    )?;
    println!(">> bindmounting app finished!");

    // chroot!
    chroot(container_path.as_str()).expect("couldn't chroot!?");
    chdir("/").expect("couldn't chdir to /!?");

    // TODO: Should totally be blocking on slirp4netns being up here...

    run_in_container(&squishfile);
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

fn touch(path: &String) -> Result<(), Box<dyn Error>> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

fn run_in_container(squishfile: &Squishfile) {
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
    std::process::Command::new(squishfile.run().command())
        .args(squishfile.run().args())
        .output()
        .unwrap();
}
