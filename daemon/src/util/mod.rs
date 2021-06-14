use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum SquishError {
    GenericError(Box<dyn std::error::Error + Send + Sync>),

    SlirpSocketCouldntBeFound,

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
