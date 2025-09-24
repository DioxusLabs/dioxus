// #![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod document;
pub mod history;

mod server_cached;
mod server_future;
mod streaming;
mod transport;

use std::prelude::rust_2024::Future;

pub use crate::server_cached::*;
pub use crate::server_future::*;
pub use crate::streaming::*;
pub use crate::transport::*;

pub mod client;

#[macro_use]
/// Error types and utilities.
pub mod error;
pub use error::*;

pub mod httperror;
pub use httperror::*;

#[derive(Clone, Default)]
pub struct DioxusServerState {}

impl DioxusServerState {
    pub fn spawn(
        &self,
        fut: impl Future<Output = axum_core::response::Response> + Send + 'static,
    ) -> Box<dyn Future<Output = axum_core::response::Response> + Send + 'static> {
        todo!()
    }
}
