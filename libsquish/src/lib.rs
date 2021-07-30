pub mod squishfile;

use std::error::Error;
use std::io::ErrorKind;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RunningContainer {
    pub id: String,
    pub name: String,
    pub pid: i32,
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
