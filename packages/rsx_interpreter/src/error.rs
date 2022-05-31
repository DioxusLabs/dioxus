use serde::{Deserialize, Serialize};

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
