// #![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod document;
pub mod history;

mod errors;
mod loader;
mod server_cached;
mod server_future;
mod streaming;
mod transport;

pub use crate::errors::*;
pub use crate::loader::*;
pub use crate::server_cached::*;
pub use crate::server_future::*;
pub use crate::streaming::*;
pub use crate::transport::*;

/// Error types and utilities.
#[macro_use]
pub mod error;
pub use error::*;

pub mod httperror;
pub use httperror::*;

/// `data-` attribute used to tag the `<script>` tags Dioxus injects into the
/// SSR output (hydration bootstrap, streaming chunk data). Hydration filters
/// these out so user-authored top-level `<script>` tags don't shift root
/// indices.
pub const HYDRATION_INJECT_MARKER: &str = "data-dioxus-hydration";
