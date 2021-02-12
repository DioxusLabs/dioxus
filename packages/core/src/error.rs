use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("No event to progress")]
    NoEvent,

    // #[error("Out of compute credits")]
    // OutOfCredits,

    // /// Used when errors need to propogate but are too unique to be typed
    // #[error("{0}")]
    // Unique(String),

    // #[error("GraphQL error: {0}")]
    // GraphQL(String),

    // // TODO(haze): Remove, or make a better error. This is pretty much useless
    // #[error("GraphQL response mismatch. Got {found} but expected {expected}")]
    // GraphQLMisMatch {
    //     expected: &'static str,
    //     found: String,
    // },

    // #[error("Difference detected in SystemTime! {0}")]
    // SystemTime(#[from] std::time::SystemTimeError),

    // #[error("Failed to parse Integer")]
    // ParseInt(#[from] std::num::ParseIntError),

    // #[error("")]
    // MissingAuthentication,

    // #[error("Failed to create experiment run: {0}")]
    // FailedToCreateExperimentRun(String),

    // #[error("Could not find shared behavior with ID {0}")]
    // MissingSharedBehavior(String),
    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),
}
