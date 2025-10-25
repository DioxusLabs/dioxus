/// Result type for geolocation operations
pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur when fetching the location.
#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// An error occurred with the Android Java environment.
    AndroidEnvironment,
    /// The user denied authorization.
    AuthorizationDenied,
    /// A network error occurred.
    Network,
    /// The function was not called from the main thread.
    NotMainThread,
    /// Location data is temporarily unavailable.
    TemporarilyUnavailable,
    /// This device does not support location data.
    PermanentlyUnavailable,
    /// An unknown error occurred.
    Unknown,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AndroidEnvironment => write!(f, "Android Java environment error"),
            Error::AuthorizationDenied => write!(f, "Location authorization denied"),
            Error::Network => write!(f, "Network error"),
            Error::NotMainThread => write!(f, "Function must be called from main thread"),
            Error::TemporarilyUnavailable => write!(f, "Location temporarily unavailable"),
            Error::PermanentlyUnavailable => write!(f, "Location not supported on this device"),
            Error::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for Error {
    fn from(_: jni::errors::Error) -> Self {
        Error::AndroidEnvironment
    }
}
