#![warn(clippy::needless_pass_by_value)]

pub mod squishfile;

use std::error::Error;
use std::io::ErrorKind;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// A currently-running container. This is effectively a three-typle of the
/// container's id, name, and pid.
#[derive(Serialize, Deserialize)]
pub struct RunningContainer {
    pub id: String,
    pub name: String,
    pub pid: i32,
}

/// Returns the current time in milliseconds since the UNIX epoch.
pub fn now() -> Result<u128, Box<dyn Error>> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis())
}

pub fn err<T, S: Into<String>>(reason: S) -> Result<T, Box<dyn Error>> {
    Err(Box::new(std::io::Error::new(
        ErrorKind::Other,
        reason.into(),
    )))
}
