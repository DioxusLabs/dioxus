// /// Response types for the browser.
// #[cfg(feature = "browser")]
// pub mod browser;

// #[cfg(feature = "generic")]
// pub mod generic;

/// Response types for Axum.
#[cfg(feature = "axum-no-default")]
pub mod http;

/// Response types for [`reqwest`].
#[cfg(feature = "reqwest")]
pub mod reqwest;
