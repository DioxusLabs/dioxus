//! Dioxus-Router
//!
//! A simple match-based router and router service for most routing needs.
//!
//! Dioxus-Router is not a *declarative* router. Instead it uses a simple parse-match
//! pattern which can be derived via a macro.
//!
//! ```rust
//! fn app(cx: Scope) -> Element {
//!     let route = use_router(&cx, |svc, path| {
//!         match path {
//!             "/about" => Route::About,
//!             _ => Route::Home,
//!         }
//!     });
//!
//!     match route {
//!         Route::Home => rsx!(cx, h1 { "Home" }),
//!         Route::About => rsx!(cx, h1 { "About" }),
//!     }
//! }
//!
//!
//!
//!
//!
//! ```
//!
//!
//!
//!
//!
//!
//!
//!
//!

mod link;
mod platform;
mod service;
mod userouter;
mod utils;

pub use link::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use service::*;
pub use userouter::*;

pub trait Routable:
    'static + Send + Clone + PartialEq + Serialize + DeserializeOwned + Default
{
}
impl<T> Routable for T where
    T: 'static + Send + Clone + PartialEq + Serialize + DeserializeOwned + Default
{
}
