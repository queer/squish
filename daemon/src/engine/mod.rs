pub mod alpine;
pub mod containers;
pub mod slirp;

use std::error::Error;
use std::process::{Command, Stdio};

use libsquish::squishfile::Squishfile;
use libsquish::SimpleCommand;
use nix::unistd::Pid;

use crate::engine::alpine::current_rootfs;

/// (container pid, slirp pid)
pub async fn spawn_container(
    id: &String,
    squishfile: Squishfile,
) -> Result<(Pid, Pid), Box<dyn Error + Send + Sync>> {
    // TODO: Ensure layers are cached
    // TODO: Pass layer names + paths to pid1

    let command = SimpleCommand::new(
        (*squishfile.run().command()).clone(),
        (*squishfile.run().args()).clone(),
    );

    // TODO: Don't hardcode this plz
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            current_rootfs().as_str(),
            "--id",
            id.as_str(),
            "--path",
            containers::path_to(&id).as_str(),
            "--command",
            // If you're going to get upset about this, just remember:
            // nftables did it first.
            // https://manpages.debian.org/testing/libnftables1/libnftables-json.5.en.html
            command
                .to_json()
                .expect("impossible (couldn't ser command!?)")
                .as_str(),
        ])
        .envs(squishfile.env())
        .output()?;

    let stdout = String::from_utf8(pid1.stdout).unwrap();
    let child_pid = stdout.trim().parse::<i32>()?;

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
        slirp.wait_with_output().await.unwrap();
    });

    for port in squishfile.ports() {
        slirp::add_port_forward(&slirp_socket_path, port.host(), port.container()).await?;
        info!(
            "{}: added port forward: {} -> {}",
            &id,
            port.host(),
            port.container()
        );
    }

    let stderr = String::from_utf8(pid1.stderr).unwrap();
    info!("{}: container spawn stderr:\n{}", &id, stderr);

    Ok((Pid::from_raw(child_pid), Pid::from_raw(slirp_pid)))
}
