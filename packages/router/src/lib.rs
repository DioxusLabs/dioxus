#![allow(warnings)]
#![doc = include_str!("../README.md")]

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
