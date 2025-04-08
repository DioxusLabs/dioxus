//! This module contains the document providers for the fullstack platform.

#[cfg(feature = "server")]
pub mod server;
use dioxus_fullstack_protocol::SerializeContextEntry;
#[cfg(feature = "server")]
pub use server::ServerDocument;
#[cfg(all(feature = "web", feature = "document"))]
pub mod web;

#[allow(unused)]
pub(crate) fn head_element_hydration_entry() -> SerializeContextEntry<bool> {
    dioxus_fullstack_protocol::serialize_context().create_entry()
}
