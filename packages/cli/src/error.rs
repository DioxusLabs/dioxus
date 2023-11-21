use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(ThisError, Debug)]
pub enum Error {
    /// Used when errors need to propogate but are too unique to be typed
    #[error("{0}")]
    Unique(String),

    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Format Error: {0}")]
    FormatError(#[from] std::fmt::Error),

    #[error("Format failed: {0}")]
    ParseError(String),

    #[error("Runtime Error: {0}")]
    RuntimeError(String),

    #[error("Failed to write error")]
    FailedToWrite,

    #[error("Build Failed: {0}")]
    BuildFailed(String),

    #[error("Cargo Error: {0}")]
    CargoError(String),

    #[error("Couldn't retrieve cargo metadata")]
    CargoMetadata(#[source] cargo_metadata::Error),

    #[error("{0}")]
    CustomError(String),

    #[error("Invalid proxy URL: {0}")]
    InvalidProxy(#[from] hyper::http::uri::InvalidUri),

    #[error("Failed to establish proxy: {0}")]
    ProxySetupError(String),

    #[error("Error proxying request: {0}")]
    ProxyRequestError(hyper::Error),

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

impl From<html_parser::Error> for Error {
    fn from(e: html_parser::Error) -> Self {
        Self::ParseError(e.to_string())
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Self::RuntimeError(e.to_string())
    }
}

impl From<dioxus_cli_config::LoadDioxusConfigError> for Error {
    fn from(e: dioxus_cli_config::LoadDioxusConfigError) -> Self {
        Self::RuntimeError(e.to_string())
    }
}

impl From<dioxus_cli_config::CargoError> for Error {
    fn from(e: dioxus_cli_config::CargoError) -> Self {
        Self::CargoError(e.to_string())
    }
}

impl From<dioxus_cli_config::CrateConfigError> for Error {
    fn from(e: dioxus_cli_config::CrateConfigError) -> Self {
        Self::RuntimeError(e.to_string())
    }
}

#[macro_export]
macro_rules! custom_error {
    ($msg:literal $(,)?) => {
        Err(Error::CustomError(format!($msg)))
    };
    ($err:expr $(,)?) => {
        Err(Error::from($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        Err(Error::CustomError(format!($fmt, $($arg)*)))
    };
}
