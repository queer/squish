pub mod alpine;
pub mod containers;
pub mod slirp;

use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::net::TcpListener;
use std::os::unix::io::FromRawFd;
use std::process::{Command, Stdio};

use libsquish::squishfile::Squishfile;
use libsquish::SyncResult;
use nix::fcntl;
use nix::sys::memfd;
use nix::unistd::{lseek, Pid, Whence};

pub const USER_AGENT: &str = "squish (https://github.com/queer/squish)";

/// (container pid, slirp pid)
/// Spawns a container, taking in the new container's ID and the squishfile
/// describing it. This function copies the squishfile to a temporary directory,
/// spawns the `pid1` binary, starts the slirp4netns process, and then applies
/// all port forwards. A tuple of pids (container, slirp4netns) is returned.
pub async fn spawn_container(id: &str, squishfile: Squishfile) -> SyncResult<(Pid, Pid)> {
    // TODO: Ensure layers are cached
    for port in squishfile.ports() {
        check_port_bind(port.host())?;
    }

    // Write squishfile into a memfd that's inherited by child processes
    let mut memfd_name = format!("squishfile-{}", id).as_bytes().to_vec();
    memfd_name.push(0);
    let memfd = memfd::memfd_create(
        CStr::from_bytes_with_nul(&memfd_name)?,
        memfd::MemFdCreateFlag::empty(),
    )?;

    // Safety: We just created the fd so we know it exists
    let mut memfd_file = unsafe { File::from_raw_fd(memfd) };
    memfd_file.write_all(
        squishfile
            .to_json()
            .expect("impossible (couldn't ser squishfile!?)")
            .as_bytes(),
    )?;
    // Turn off FD_CLOEXEC
    let old_flags = fcntl::fcntl(memfd, fcntl::FcntlArg::F_GETFL)?;
    let fixed_flags = old_flags & !(fcntl::FdFlag::FD_CLOEXEC.bits());
    fcntl::fcntl(
        memfd,
        fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::from_bits_truncate(fixed_flags)),
    )?;
    // Seek to zero
    lseek(memfd, 0, Whence::SeekSet)?;

    // Spawn stuff
    debug!("{}: pid1 setup", &id);
    let base_version = alpine::VERSION.to_string();
    let base_arch = alpine::ARCH.to_string();
    // TODO: Allow not having an alpine base image for "FROM scratch"-equiv containers
    let alpine_version = match squishfile.layers().get("alpine") {
        Some(version) => version
            .version()
            .as_ref()
            .expect("No alpine version present!?"),
        None => &base_version,
    };
    alpine::download_base_image(alpine_version, &base_arch).await?;
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            alpine::current_rootfs(alpine_version, &base_arch).as_str(),
            "--id",
            id,
            "--path",
            containers::path_to(id).as_str(),
            "--squishfile-memfd",
            format!("{}", memfd).as_str(),
        ])
        .envs(squishfile.env())
        .output()?;

    let stderr = String::from_utf8(pid1.stderr).unwrap();
    debug!("{}: container spawn stderr:\n{}", &id, stderr);

    let stdout = String::from_utf8(pid1.stdout).unwrap();
    let child_pid = stdout.trim().parse::<i32>()?;

    debug!("{}: slirp4netns setup", &id);
    let slirp_socket_path = format!("/tmp/slirp4netns-{}.sock", &id);
    let slirp = tokio::process::Command::new("cache/slirp4netns")
        .args(vec![
            "--configure",
            "--mtu=65520",
            "--disable-host-loopback",
            "--api-socket",
            slirp_socket_path.as_str(),
            format!("{}", child_pid).as_str(),
            "tap0",
        ])
        // TODO: Should we be capturing these logs?
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let slirp_pid = slirp.id().expect("no slirp4netns pid!?") as i32;

    tokio::spawn(async move {
        // debug!("{}: await slirp4netns exit", &id);
        let _output = slirp.wait_with_output().await.unwrap();
        // let stdout = String::from_utf8(output.stdout).unwrap();
        // let stderr = String::from_utf8(output.stderr).unwrap();
        // debug!("{}: s4nns exit: {}:\n--------\nstdout:\n{}\n--------\nstderr:\n{}\n--------", &id, output.status, stdout, stderr);
    });

    debug!("{}: port forward setup", &id);
    for port in squishfile.ports() {
        slirp::add_port_forward(&slirp_socket_path, port.host(), port.container()).await?;
        debug!(
            "{}: added port forward: {} -> {}",
            &id,
            port.host(),
            port.container()
        );
    }

    Ok((Pid::from_raw(child_pid), Pid::from_raw(slirp_pid)))
}

fn check_port_bind(port: &u16) -> SyncResult<()> {
    TcpListener::bind(("127.0.0.1".to_string(), *port))
        .map(|_| ())
        .map_err(|e| e.into())
}
