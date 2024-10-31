//! Incremental file based incremental rendering

#![allow(non_snake_case)]

mod config;
mod document;
mod error;
mod freshness;

// #[cfg(not(target_arch = "wasm32"))]
// mod fs_cache;

mod cache;
pub use cache::*;

mod spawn;
pub use spawn::*;

mod ssr;
pub use ssr::*;

mod index;
pub use index::*;
mod state;
pub use state::*;
mod chunk;
pub use chunk::*;
mod ext;
pub use ext::*;
pub mod launch;
mod memory_cache;
pub use error::*;
mod template;
pub use template::*;
mod stream;
pub use stream::*;
mod streaming;
pub use streaming::*;
mod render;
pub use render::*;
mod server;
pub use server::*;
mod serve_config;
pub use serve_config::*;
mod server_context;
pub use server_context::*;
mod mutation_writer;
pub use mutation_writer::*;

use std::time::Duration;

pub mod prelude {
    // pub use super::*;
    pub use super::IncrementalRendererConfig;
}

use chrono::Utc;
pub use config::*;
pub use freshness::*;
