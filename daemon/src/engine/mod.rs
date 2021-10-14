pub mod alpine;
pub mod containers;
pub mod slirp;

use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

use libsquish::squishfile::Squishfile;
use nix::unistd::Pid;

/// (container pid, slirp pid)
/// Spawns a container, taking in the new container's ID and the squishfile
/// describing it. This function copies the squishfile to a temporary directory,
/// spawns the `pid1` binary, starts the slirp4netns process, and then applies
/// all port forwards. A tuple of pids (container, slirp4netns) is returned.
pub async fn spawn_container(
    id: &String,
    squishfile: Squishfile,
) -> Result<(Pid, Pid), Box<dyn Error + Send + Sync>> {
    // TODO: Ensure layers are cached

    // Write squishfile into /tmp to avoid leaking it to `ps`
    let temp_path = format!("/tmp/{}.squishfile.toml", id);
    let mut file = File::create(&temp_path)?;
    file.write_all(
        squishfile
            .to_json()
            .expect("impossible (couldn't ser squishfile!?)")
            .as_bytes(),
    )?;

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
    alpine::download_base_image(&alpine_version, &base_arch).await?;
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            alpine::current_rootfs(&alpine_version, &base_arch).as_str(),
            "--id",
            id.as_str(),
            "--path",
            containers::path_to(&id).as_str(),
            "--squishfile",
            temp_path.as_str(),
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
        // TODO: lol
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
