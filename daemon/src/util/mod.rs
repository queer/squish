use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum SquishError {
    NixError(nix::Error),

    AlpineManifestInvalid,
    AlpineManifestMissing,
    AlpineManifestFileMissing,
}

impl Display for SquishError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        todo!()
    }
}

impl warp::reject::Reject for SquishError {}

impl Error for SquishError {}

pub fn now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis()
}
