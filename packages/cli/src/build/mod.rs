//! The core build module for `dx`, enabling building, bundling, and runtime hot-patching of Rust
//! applications. This module defines the entire end-to-end build process, including bundling for
//! all major platforms including Mac, Windows, Linux, iOS, Android, and WebAssembly.
//!
//! The bulk of the builder code is contained within the [`request`] module which establishes the
//! arguments and flow of the build process. The [`context`] module contains the context for the build
//! including status updates and build customization. The [`patch`] module contains the logic for
//! hot-patching Rust code through binary analysis and a custom linker. The [`builder`] module contains
//! the management of the ongoing build and methods to open the build as a running app.

mod assets;
mod builder;
pub(crate) mod cache;
mod context;
mod manifest;
mod patch;
mod pre_render;
mod request;
mod tools;

pub(crate) use assets::*;
pub(crate) use builder::*;
pub(crate) use cache::*;
pub(crate) use context::*;
pub(crate) use manifest::*;
pub(crate) use patch::*;
pub(crate) use pre_render::*;
pub(crate) use request::*;
pub(crate) use tools::*;
