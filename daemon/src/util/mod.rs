use std::fmt::Display;

#[derive(Debug)]
pub enum SquishError {
    NixError(nix::Error),
}

impl Display for SquishError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        todo!()
    }
}

impl warp::reject::Reject for SquishError {}

// impl From<SquishError> for Rejection {
//     fn from(other: SquishError) -> Self {
//         warp::reject::custom(other)
//     }
// }
