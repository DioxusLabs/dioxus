use std::error::Error;
use std::fmt::Display;

/// Represents an error when evaluating JavaScript
#[derive(Debug)]
#[non_exhaustive]
pub enum EvalError {
    /// The platform does not support evaluating JavaScript.
    Unsupported,

    /// The provided JavaScript has already been ran.
    Finished,

    /// The provided JavaScript is not valid and can't be ran.
    InvalidJs(String),

    /// Represents an error communicating between JavaScript and Rust.
    Communication(String),

    /// Represents an error deserializing the result of an eval
    Deserialization(serde_json::Error),
}

impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Unsupported => write!(f, "EvalError::Unsupported - eval is not supported on the current platform"),
            EvalError::Finished => write!(f, "EvalError::Finished - eval has already ran"),
            EvalError::InvalidJs(_) => write!(f, "EvalError::InvalidJs - the provided javascript is invalid"),
            EvalError::Communication(_) => write!(f, "EvalError::Communication - there was an error trying to communicate with between javascript and rust"),
            EvalError::Deserialization(_) => write!(f, "EvalError::Deserialization - there was an error trying to deserialize the result of an eval"),
        }
    }
}

impl Error for EvalError {}
