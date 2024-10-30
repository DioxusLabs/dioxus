pub type Result<T> = std::result::Result<T, Error>;

pub type IncrementalRendererError = Error;

/// An error that can occur while rendering a route or retrieving a cached route.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An formatting error occurred while rendering a route.
    #[error("RenderError: {0}")]
    RenderError(#[from] std::fmt::Error),

    /// An IO error occurred while rendering a route.
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    /// An IO error occurred while rendering a route.
    #[error("Other: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
