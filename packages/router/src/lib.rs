#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod hooks {
    mod use_route;
    mod use_router;
    pub use use_route::*;
    pub use use_router::*;
}
pub use hooks::*;

mod components {
    #![allow(non_snake_case)]

    mod link;
    mod redirect;
    mod route;
    mod router;

    pub use link::*;
    pub use redirect::*;
    pub use route::*;
    pub use router::*;
}
pub use components::*;

mod cfg;
mod routecontext;
mod service;

pub use routecontext::*;
pub use service::*;
