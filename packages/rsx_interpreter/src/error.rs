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
