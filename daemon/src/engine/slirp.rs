use std::error::Error;
use std::fs;
use std::fs::Permissions;
use std::io::Write;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;

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
