pub mod alpine;
pub mod containers;

use std::process::Command;

use nix::unistd::Pid;

use crate::engine::alpine::current_rootfs;

pub fn spawn_container(id: String) -> Result<Pid, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Don't hardcode this plz
    let pid = Command::new("target/debug/pid1").args(vec![
        "--rootfs",
        current_rootfs().as_str(),
        "--id",
        id.as_str(),
    ]).output()?;

    let stdout = String::from_utf8(pid.stdout).unwrap();
    Ok(Pid::from_raw(stdout.trim().parse::<i32>().unwrap()))
}
