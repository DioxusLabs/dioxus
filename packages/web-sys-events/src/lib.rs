//! Web-sys event implementations for Dioxus
//!
//! This crate provides the web-sys event implementations that can be shared
//! between dioxus-web and dioxus-desktop.

mod data_transfer;
pub mod events;
pub mod files;

pub use data_transfer::*;
pub use events::*;
pub use files::*;
