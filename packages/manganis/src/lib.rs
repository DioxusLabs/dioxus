#![doc = include_str!("../../../README.md")]
#![deny(missing_docs)]

#[cfg(feature = "macro")]
pub use manganis_macro::*;

mod folder;
pub use folder::*;

mod images;
pub use images::*;

mod builder;
pub use builder::*;
