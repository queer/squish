use crate::util::SquishError;

use std::error::Error;
use std::fs;
use std::fs::Permissions;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use tokio::time::sleep;

pub async fn download_slirp4netns() -> Result<&'static str, Box<dyn Error>> {
    let output_path = "cache/slirp4netns";
    if Path::new(output_path).exists() {
        return Ok(output_path);
    }
    // TODO: Better handling
    let slirp_bytes = reqwest::get("https://github.com/rootless-containers/slirp4netns/releases/download/v1.1.10/slirp4netns-x86_64").await?.bytes().await?;
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)?;
    output_file.write(&slirp_bytes)?;
    fs::set_permissions(output_path, Permissions::from_mode(0o755))?;
    eprintln!("{:o}", output_file.metadata()?.permissions().mode());
    Ok(output_path)
}

pub async fn slirp_exec(
    slirp_socket_path: &String,
    command: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    info!("connecting to: {}", slirp_socket_path);
    let mut attempts: u8 = 0;
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
