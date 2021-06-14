pub mod alpine;
pub mod containers;
pub mod slirp;

use std::error::Error;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::Duration;

use nix::unistd::Pid;
use tokio::time::sleep;

use crate::{engine::alpine::current_rootfs, util::SquishError};

/// (container pid, slirp pid)
pub async fn spawn_container(id: String) -> Result<(Pid, Pid), Box<dyn Error + Send + Sync>> {
    // TODO: Don't hardcode this plz
    let pid1 = Command::new("target/debug/pid1")
        .args(vec![
            "--rootfs",
            current_rootfs().as_str(),
            "--id",
            id.as_str(),
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

    let add_res = slirp_exec(&slirp_socket_path, r#"
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

    let list_res = slirp_exec(&slirp_socket_path, r#"
        {
            "execute": "list_hostfwd"
        }
    "#).await?;
    info!("slirp said: {}", list_res);

    let stderr = String::from_utf8(pid1.stderr).unwrap();
    info!("container spawn stderr:\n{}", stderr);

    Ok((Pid::from_raw(child_pid), Pid::from_raw(slirp_pid)))
}

async fn slirp_exec(
    slirp_socket_path: &String,
    command: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    info!("connecting to: {}", slirp_socket_path);
    let mut attempts = 0;
    let mut slirp_socket;
    loop {
        match UnixStream::connect(slirp_socket_path) {
            Ok(stream) => {
                slirp_socket = stream;
                break;
            }
            Err(_) => {}
        }
        attempts += 1;
        if attempts > 100 {
            return Err(Box::new(SquishError::SlirpSocketCouldntBeFound));
        }
        sleep(Duration::from_millis(1)).await;
    }
    debug!("slirp socket connected (attempts={})", attempts);
    slirp_socket.write_all(command.as_bytes())?;
    let mut res = String::new();
    slirp_socket.read_to_string(&mut res)?;
    Ok(res)
}
