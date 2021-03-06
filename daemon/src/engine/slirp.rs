use crate::engine::USER_AGENT;
use crate::util::SquishError;

use std::fs;
use std::fs::Permissions;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use libsquish::SyncResult;
use tokio::time::sleep;

const URL: &str = "https://github.com/rootless-containers/slirp4netns/releases/download/v1.1.11/slirp4netns-x86_64";

/// Downloads the current slirp4netns binary. This caches in the same directory
/// as the Alpine rootfs images.
pub async fn download_slirp4netns() -> SyncResult<&'static str> {
    // TODO: Version this
    let output_path = "cache/slirp4netns";
    if Path::new(output_path).exists() {
        info!("slirp4netns binary already exists, not downloading again");
        return Ok(output_path);
    }
    info!("downloading slirp4netns binary from {}", URL);
    // TODO: Refactor this to reuse code from alpine / layers where possible
    let slirp_bytes = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(URL)
        .send()
        .await?
        .bytes()
        .await?;
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)?;
    output_file.write_all(&slirp_bytes)?;
    fs::set_permissions(output_path, Permissions::from_mode(0o755))?;
    // eprintln!("{:o}", output_file.metadata()?.permissions().mode());
    Ok(output_path)
}

/// Adds a port-forward to the given slirp4netns instance via its socket.
pub async fn add_port_forward(socket: &str, host: &u16, container: &u16) -> SyncResult<String> {
    slirp_exec(
        socket,
        format!(
            r#"
        {{
            "execute": "add_hostfwd",
            "arguments": {{
                "proto": "tcp",
                "host_ip": "127.0.0.1",
                "host_port": {},
                "guest_port": {}
            }}
        }}
    "#,
            host, container
        )
        .as_str(),
    )
    .await
}

/// Executes a slirp4netns command over the given socket.
pub async fn slirp_exec(slirp_socket_path: &str, command: &str) -> SyncResult<String> {
    info!("connecting to: {}", slirp_socket_path);
    let mut attempts: u8 = 0;
    let mut slirp_socket;
    loop {
        if let Ok(s) = UnixStream::connect(slirp_socket_path) {
            slirp_socket = s;
            break;
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
