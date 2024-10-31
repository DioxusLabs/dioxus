use http::StatusCode;

pub type Result<T> = std::result::Result<T, Error>;

pub type IncrementalRendererError = Error;

/// An error that can occur while rendering a route or retrieving a cached route.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An formatting error occurred while rendering a route.
    #[error("RenderError: {0}")]
    RenderError(#[from] std::fmt::Error),

    //
    #[error("Crashed while rendering")]
    Crash(),

    ///
    #[error("Axum error")]
    Http(http::StatusCode),

    /// An IO error occurred while rendering a route.
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    /// The client disconnected before the stream could be completed.
    #[error("Disconnected")]
    Disconnected(#[from] futures_channel::mpsc::SendError),

    /// An IO error occurred while rendering a route.
    #[error("Other: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
