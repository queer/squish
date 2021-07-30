pub mod alpine;
pub mod containers;
pub mod slirp;

use std::error::Error;
use std::process::{Command, Stdio};

use libsquish::squishfile::Squishfile;
use nix::unistd::Pid;

use crate::engine::alpine::current_rootfs;

/// (container pid, slirp pid)
pub async fn spawn_container(
    id: &String,
    squishfile: Squishfile,
) -> Result<(Pid, Pid), Box<dyn Error + Send + Sync>> {
    // TODO: Ensure layers are cached
    // TODO: Pass layer names + paths to pid1

    // TODO: Don't hardcode this plz
    debug!("{}: pid1 setup", &id);
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            current_rootfs().as_str(), // TODO: Allow just not having a rootfs
            "--id",
            id.as_str(),
            "--path",
            containers::path_to(&id).as_str(),
            "--squishfile",
            // If you're going to get upset about this, just remember:
            // nftables did it first.
            // https://manpages.debian.org/testing/libnftables1/libnftables-json.5.en.html
            squishfile
                .to_json()
                .expect("impossible (couldn't ser command!?)")
                .as_str(),
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

    info!("{}: port forward setup", &id);
    for port in squishfile.ports() {
        slirp::add_port_forward(&slirp_socket_path, port.host(), port.container()).await?;
        info!(
            "{}: added port forward: {} -> {}",
            &id,
            port.host(),
            port.container()
        );
    }

    Ok((Pid::from_raw(child_pid), Pid::from_raw(slirp_pid)))
}
