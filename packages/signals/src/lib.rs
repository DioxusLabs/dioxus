#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
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
mod dependency;
pub use dependency::*;
