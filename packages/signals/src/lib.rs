#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod rt;
pub use rt::*;
mod effect;
pub use effect::*;
mod impls;
mod selector;
pub use selector::*;
pub(crate) mod signal;
pub use signal::*;
