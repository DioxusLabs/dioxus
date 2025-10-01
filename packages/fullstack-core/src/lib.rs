// #![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod document;
pub mod history;

mod server_cached;
mod server_future;
mod streaming;
mod transport;

pub use crate::server_cached::*;
pub use crate::server_future::*;
pub use crate::streaming::*;
pub use crate::transport::*;

/// Error types and utilities.
#[macro_use]
pub mod error;
pub use error::*;

pub mod httperror;
pub use httperror::*;

#[derive(Clone, Default)]
pub struct DioxusServerState {}
