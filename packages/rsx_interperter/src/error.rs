#[derive(Debug)]
pub enum Error {
    ParseError(syn::Error),
    RecompileRequiredError(RecompileReason),
}

#[derive(Debug)]
pub enum RecompileReason {
    CapturedVariable(String),
    CapturedExpression(String),
    CapturedComponent(String),
    CapturedListener(String),
}
