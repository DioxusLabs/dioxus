#![allow(non_snake_case)]

mod config;
pub use config::*;
pub mod launch;

#[cfg(feature = "server")]
pub(crate) mod ssg;

/// A prelude of commonly used items in static generation apps.
pub mod prelude {
    pub use dioxus_fullstack::prelude::*;
}
