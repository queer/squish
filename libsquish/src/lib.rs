#![warn(clippy::needless_pass_by_value)]

pub mod squishfile;

use std::error::Error;
use std::io::ErrorKind;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub type SyncResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// A currently-running container. This is effectively a three-typle of the
/// container's id, name, and pid.
#[derive(Serialize, Deserialize)]
pub struct RunningContainer {
    pub id: String,
    pub name: String,
    pub pid: i32,
}

/// Returns the current time in milliseconds since the UNIX epoch.
pub fn now() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis())
}

pub fn err<T, S: Into<String>>(reason: S) -> Result<T> {
    Err(Box::new(std::io::Error::new(
        ErrorKind::Other,
        reason.into(),
    )))
}
