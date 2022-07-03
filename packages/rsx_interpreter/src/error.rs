use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::CodeLocation;

/// An error produced when interperting the rsx
#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    ParseError(ParseError),
    RecompileRequiredError(RecompileReason),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecompileReason {
    CapturedVariable(String),
    CapturedExpression(String),
    CapturedComponent(String),
    CapturedListener(String),
    CapturedAttribute(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub location: CodeLocation,
}

impl ParseError {
    pub fn new(error: syn::Error, mut location: CodeLocation) -> Self {
        let message = error.to_string();
        let syn_call_site = error.span().start();
        location.line += syn_call_site.line as u32;
        if syn_call_site.line == 0 {
            location.column += syn_call_site.column as u32;
        } else {
            location.column = syn_call_site.column as u32;
        }
        location.column += 1;
        ParseError { message, location }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ParseError(error) => write!(
                f,
                "parse error:\n--> at {}:{}:{}\n\t{:?}\n",
                error.location.file_path, error.location.line, error.location.column, error.message
            ),
            Error::RecompileRequiredError(reason) => {
                write!(f, "recompile required: {:?}\n", reason)
            }
        }
    }
}
