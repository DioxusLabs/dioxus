//! Internal error handling for Dioxus
//!
//!

use thiserror::Error as ThisError;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Fatal Internal Error: {0}")]
    FatalInternal(&'static str),

    #[error("Context is missing")]
    MissingSharedContext,

    #[error("No event to progress")]
    NoEvent,

    #[error("Wrong Properties Type")]
    WrongProps,

    #[error("The component failed to return VNodes")]
    ComponentFailed,

    #[error("Base scope has not been mounted yet")]
    NotMounted,

    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),
}
