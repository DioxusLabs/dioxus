#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod build;
mod bundle_utils;
mod cli;
mod config;
mod dioxus_crate;
mod dx_build_info;
mod error;
mod fastfs;
mod filemap;
mod logging;
mod metadata;
mod platform;
mod rustup;
mod serve;
mod settings;
mod wasm_bindgen;

pub use build::*;
pub use cli::*;
pub use config::*;
pub use dioxus_crate::*;
pub use dioxus_dx_wire_format::*;
pub use error::*;
pub use filemap::*;
pub use logging::*;
pub use platform::*;
pub use rustup::*;
pub use settings::*;
