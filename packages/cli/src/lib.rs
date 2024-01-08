#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub const DIOXUS_CLI_VERSION: &str = "0.4.1";

mod assets;
pub mod builder;
pub mod server;
pub mod tools;

pub use builder::*;

pub mod cli;
pub use cli::*;

pub mod error;
pub use error::*;

pub mod logging;
pub use logging::*;

#[cfg(feature = "plugin")]
pub mod plugin;
