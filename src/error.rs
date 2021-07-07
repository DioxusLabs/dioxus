use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(ThisError, Debug)]
pub enum Error {
    /// Used when errors need to propogate but are too unique to be typed
    #[error("{0}")]
    Unique(String),

    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Failed to write error")]
    FailedToWrite,

    #[error("Failed to write error")]
    CargoError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Unique(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Unique(s)
    }
}
