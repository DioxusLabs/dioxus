#![doc = include_str!("readme.md")]
#![allow(non_snake_case)]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod config;
pub use config::*;
pub mod launch;

#[cfg(feature = "server")]
pub(crate) mod ssg;

/// A prelude of commonly used items in static generation apps.
pub mod prelude {
    pub use dioxus_fullstack::prelude::*;
}
