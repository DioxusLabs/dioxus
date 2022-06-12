use serde::{Deserialize, Serialize};

/// An error produced when interperting the rsx
#[derive(Debug)]
pub enum Error {
    ParseError(syn::Error),
    RecompileRequiredError(RecompileReason),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecompileReason {
    CapturedVariable(String),
    CapturedExpression(String),
    CapturedComponent(String),
    CapturedListener(String),
}
