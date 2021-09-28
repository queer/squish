use crate::util::SquishError;

use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use yaml_rust::{Yaml, YamlLoader};

/// The current version of Alpine that this squishd knows about.
/// TODO: Dynamic version finding
pub const VERSION: &'static str = "3.14";
/// The architecture of Alpine that this squishd knows about. Maybe this will
/// support ARM etc. in the future.
pub const ARCH: &'static str = "x86_64";

/// The rootfs directory. This is the directory that Alpine rootfs images are
/// cached in.
pub fn rootfs_directory() -> &'static str {
    "cache/alpine/rootfs"
}

/// The path to the current rootfs tarball. This is
/// `rootfs_directory()/alpine-rootfs-{VERSION}-{ARCH}.tar.gz`.
pub fn current_rootfs_tarball(version: &String, arch: &String) -> String {
    format!(
        "{}/alpine-rootfs-{}-{}.tar.gz",
        rootfs_directory(),
        version,
        arch
    )
}

/// The current rootfs. This is determined by the baked-in version / arch, and
/// resolves to a path under the main rootfs directory.
/// TODO: Dynamic version finding
pub fn current_rootfs(version: &String, arch: &String) -> String {
    format!("{}/alpine-rootfs-{}-{}", rootfs_directory(), version, arch)
}

/// The base URL to download Alpine rootfs images from.
/// TODO: Use a mirror list properly
pub fn base_url(version: &String, arch: &String) -> String {
    format!(
        "https://cz.alpinelinux.org/alpine/v{}/releases/{}",
        version, arch
    )
}

/// Download the base Alpine rootfs image. This will download and cache the
/// rootfs image from a mirror (based on `base_url()`).
pub async fn download_base_image(
    version: &String,
    arch: &String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if Path::new(&current_rootfs_tarball(&version, &arch)).exists() {
        info!("rootfs tarball already exists, not downloading again");
        return Ok(());
    }
    let manifest_url = format!("{}/latest-releases.yaml", base_url(&version, &arch));
    debug!("downloading alpine minirootfs from {}", &manifest_url);
    let manifest_text = reqwest::get(manifest_url).await?.text().await?;

    let docs = YamlLoader::load_from_str(manifest_text.as_str())?;
    let manifest = &docs[0];
    if let Some(vec) = manifest.as_vec() {
        let maybe_rootfs_manifest = vec.iter().find(|yaml| match yaml["flavor"].as_str() {
            Some("alpine-minirootfs") => true,
            _ => false,
        });
        if let Some(rootfs_manifest) = maybe_rootfs_manifest {
            info!("found alpine minirootfs! downloading...");
            let tarball = download_rootfs(rootfs_manifest, &version, &arch).await?;
            extract_tarball(tarball, current_rootfs(&version, &arch))?;
            setup_rootfs(current_rootfs(&version, &arch))
        } else {
            Err(Box::new(SquishError::AlpineManifestMissing))
        }
    } else {
        Err(Box::new(SquishError::AlpineManifestInvalid))
    }
}

async fn download_rootfs(
    rootfs_manifest: &Yaml,
    version: &String,
    arch: &String,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match rootfs_manifest["file"].as_str() {
        Some(rootfs_filename) => {
            // minirootfs is a ~3MB tarball, so we can afford to hold
            // it all in memory.
            let rootfs_url = format!("{}/{}", base_url(version, arch), rootfs_filename);

            let download_response = reqwest::get(rootfs_url).await?;
            let rootfs_bytes = download_response.bytes().await?;

            let output_path = current_rootfs_tarball(version, arch);
            debug!("downloading alpine minirootfs into {}", &output_path);
            fs::create_dir_all(&rootfs_directory())?;
            let mut output_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output_path)?;
            output_file.write(&rootfs_bytes)?;
            Ok(output_path)
        }
        None => return Err(Box::new(SquishError::AlpineManifestFileMissing)),
    }
}

fn extract_tarball(path: String, target_path: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("extracting alpine rootfs from {} to {}", path, target_path);
    let tarball = fs::File::open(path)?;
    let tar = flate2::read::GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(target_path)?;
    Ok(())
}

fn setup_rootfs(rootfs: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    // devices
    info!("setting up dummy devices");
    File::create(format!("{}/dev/null", rootfs))?;
    File::create(format!("{}/dev/zero", rootfs))?;
    File::create(format!("{}/dev/random", rootfs))?;
    File::create(format!("{}/dev/urandom", rootfs))?;
    File::create(format!("{}/dev/console", rootfs))?;
    fs::create_dir_all(format!("{}/dev/shm", rootfs))?;
    fs::create_dir_all(format!("{}/dev/pts", rootfs))?;

    // mountable dirs
    info!("setting up /proc and /sys stubs");
    fs::create_dir_all(format!("{}/proc", rootfs))?;
    fs::create_dir_all(format!("{}/sys", rootfs))?;

    // squish layers
    info!("setting up squish layer stubs");
    fs::create_dir_all(format!("{}/app", rootfs))?;
    fs::create_dir_all(format!("{}/sdk", rootfs))?;

    // networking
    info!("setting up resolv.conf");
    let mut resolv = File::create(format!("{}/etc/resolv.conf", rootfs))?;
    resolv.write("nameserver 10.0.2.3".as_bytes())?; // slirp4netns
    Ok(())
}
