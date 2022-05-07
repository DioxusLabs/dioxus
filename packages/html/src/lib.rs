#![allow(non_snake_case)]

//! # Dioxus Namespace for HTML
//!
//! This crate provides a set of compile-time correct HTML elements that can be used with the Rsx and Html macros.
//! This system allows users to easily build new tags, new types, and customize the output of the Rsx and Html macros.
//!
//! An added benefit of this approach is the ability to lend comprehensive documentation on how to use these elements inside
//! of the Rsx and Html macros. Each element comes with a substantial amount of documentation on how to best use it, hopefully
//! making the development cycle quick.
//!
//! All elements are used as zero-sized unit structs with trait impls.
//!
//! Currently, we don't validate for structures, but do validate attributes.

mod elements;
mod events;
pub mod geometry;
mod global_attributes;
mod input;
#[cfg(feature = "wasm-bind")]
mod web_sys_bind;

pub use elements::*;
pub use events::*;
pub use global_attributes::*;
pub use input::*;
