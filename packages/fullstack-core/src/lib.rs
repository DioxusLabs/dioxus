// #![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod document;
pub mod history;

mod errors;
mod loader;
mod server_cached;
mod server_future;
mod streaming;
mod transport;

use std::{hash::Hash, marker::PhantomData, sync::Arc};

pub use crate::errors::*;
pub use crate::loader::*;
pub use crate::server_cached::*;
pub use crate::server_future::*;
pub use crate::streaming::*;
pub use crate::transport::*;

/// Error types and utilities.
#[macro_use]
pub mod error;
pub use error::*;

pub mod httperror;
use http::Extensions;
pub use httperror::*;

#[derive(Clone)]
pub struct ServerFnState {
    _priv: PhantomData<()>,
}

impl ServerFnState {
    pub fn new() -> Self {
        Self { _priv: PhantomData }
    }

    pub fn get<T: Send + Sync + Clone + 'static>(&self) -> T {
        todo!()
        // self.try_get()
        //     .expect("Requested type not found in ServerFnState")
    }

    pub fn try_get<T: Send + Sync + Clone + 'static>(&self) -> Option<T> {
        todo!()
        // self.extensions.get::<T>().cloned()
    }
}
