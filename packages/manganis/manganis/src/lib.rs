//! Manganis is a Rust library for bundling assets into a final binary.

#![deny(missing_docs)]

#[cfg(feature = "macro")]
pub use manganis_macro::*;

mod folder;
pub use folder::*;

mod images;
pub use images::*;

mod builder;
pub use builder::*;
