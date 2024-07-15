#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod assets;
pub mod dx_build_info;
pub mod serve;
pub mod tools;

pub mod cli;
pub use cli::*;

pub mod error;
pub use error::*;

pub(crate) mod builder;

mod dioxus_crate;
pub use dioxus_crate::*;

mod settings;
pub(crate) use settings::*;

pub(crate) mod metadata;
