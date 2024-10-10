//! This module contains the document providers for the fullstack platform.

#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub use server::ServerDocument;
#[cfg(all(feature = "web", feature = "document"))]
pub mod web;
