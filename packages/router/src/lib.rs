#![allow(warnings)]
//! Dioxus-Router
//!
//! A simple match-based router and router service for most routing needs.
//!
//! Dioxus-Router is not a *declarative* router. Instead it uses a simple parse-match
//! pattern which can be derived via a macro.
//!
//! ```rust
//! fn app(cx: Scope) -> Element {
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

mod hooks {
    mod use_route;
    pub use use_route::*;
}
pub use hooks::*;

mod components {
    #![allow(non_snake_case)]

    mod router;
    pub use router::*;

    mod route;
    pub use route::*;

    mod link;
    pub use link::*;
}
pub use components::*;

mod platform;
mod routecontext;
mod service;
mod utils;

pub use routecontext::*;
pub use service::*;
