//! Runtime support for the Dioxus inspector tooling.

#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod protocol;

pub use dioxus_inspector_macros::inspector;

#[cfg(feature = "client")]
pub use client::InspectorClient;

#[cfg(feature = "server")]
pub use protocol::{IdeKind, InspectorRequest};

/// Prelude containing the most common exports.
pub mod prelude {
    pub use crate::inspector;
}
