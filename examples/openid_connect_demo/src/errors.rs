use openidconnect::{core::CoreErrorResponseType, url, RequestTokenError, StandardErrorResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Discovery error: {0}")]
    OpenIdConnect(
        #[from] openidconnect::DiscoveryError<openidconnect::reqwest::Error<reqwest::Error>>,
    ),
    #[error("Parsing error: {0}")]
    Parse(#[from] url::ParseError),
    #[error("Request token error: {0}")]
    RequestToken(
        #[from]
        RequestTokenError<
            openidconnect::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<CoreErrorResponseType>,
        >,
    ),
}
