#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod hooks {
    mod use_param;
    mod use_query;
    mod use_route;
    pub use use_param::*;
    pub use use_query::*;
    pub use use_route::*;
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

mod routecontext;
mod service;
mod utils;

pub use routecontext::*;
pub use service::*;

mod error {
    pub type Result<T> = std::result::Result<T, Error>;

    pub enum Error {}
}
pub use error::*;
