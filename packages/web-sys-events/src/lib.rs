//! Web-sys event implementations for Dioxus
//!
//! This crate provides the web-sys event implementations that can be shared
//! between dioxus-web and dioxus-desktop.

mod bridge;
mod data_transfer;
pub mod events;
pub mod files;
mod queue_mounted_events;

pub use bridge::*;
pub use data_transfer::*;
pub use events::*;
pub use files::*;
pub use queue_mounted_events::*;
