pub mod squishfile;

use std::error::Error;
use std::io::ErrorKind;
use std::time::SystemTime;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RunningContainer {
    pub id: String,
    pub name: String,
    pub pid: i32,
}

#[derive(Serialize, Deserialize, Getters)]
pub struct SimpleCommand {
    command: String,
    args: Vec<String>,
}

impl SimpleCommand {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }

    pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
        serde_json::to_string(&self).map_err(|e| e.into())
    }

    pub fn from_json<'a, S: Into<&'a str>>(json: S) -> Result<Self, Box<dyn Error>> {
        serde_json::from_str(json.into()).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

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
