#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod history;
mod hooks;
mod streaming;

pub mod prelude {
    //! A prelude of commonly used items in dioxus-fullstack-hooks.

    pub use crate::hooks::*;
    pub use crate::streaming::*;
}
