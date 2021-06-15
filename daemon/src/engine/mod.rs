pub mod alpine;
pub mod containers;
pub mod slirp;

use std::error::Error;
use std::process::{Command, Stdio};

use nix::unistd::Pid;

use crate::engine::alpine::current_rootfs;

/// (container pid, slirp pid)
pub async fn spawn_container(id: String) -> Result<(Pid, Pid), Box<dyn Error + Send + Sync>> {
    // TODO: Don't hardcode this plz
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            current_rootfs().as_str(),
            "--id",
            id.as_str(),
            "--path",
            containers::path_to(&id).as_str(),
        ])
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

    let add_res = slirp::slirp_exec(&slirp_socket_path, r#"
        {
            "execute": "add_hostfwd",
            "arguments": {
                "proto": "tcp",
                "host_ip": "127.0.0.1",
                "host_port": 42069,
                "guest_port": 2000
            }
        }
    "#).await?;
    info!("slirp said: {}", add_res);

    let list_res = slirp::slirp_exec(&slirp_socket_path, r#"
        {
            "execute": "list_hostfwd"
        }
    "#).await?;
    info!("slirp said: {}", list_res);

    let stderr = String::from_utf8(pid1.stderr).unwrap();
    info!("container spawn stderr:\n{}", stderr);

    Ok((Pid::from_raw(child_pid), Pid::from_raw(slirp_pid)))
}
