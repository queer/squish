use std::any::{Any, TypeId};
use std::{error::Error, fmt::Display};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum SquishError {
    GenericError(Box<dyn std::error::Error + Send + Sync>),

    SlirpSocketCouldntBeFound,

    AlpineManifestInvalid,
    AlpineManifestMissing,
    AlpineManifestFileMissing,

    CgroupDelegationInvalid,
    CgroupNoMoreSlices,
}

impl Display for SquishError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        todo!()
    }
}

impl warp::reject::Reject for SquishError {}

impl Error for SquishError {}

// https://stackoverflow.com/a/52005668
pub trait SameType
where
    Self: Any,
{
    fn same_type<U: ?Sized + Any>(&self) -> bool {
        TypeId::of::<Self>() == TypeId::of::<U>()
    }
}

impl<T: ?Sized + Any> SameType for T {}
