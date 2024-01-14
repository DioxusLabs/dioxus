#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
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
#[cfg(feature = "hot-reload-context")]
pub use elements::HtmlCtx;
#[cfg(feature = "html-to-rsx")]
pub use elements::{map_html_attribute_to_rsx, map_html_element_to_rsx};
pub mod events;
pub(crate) mod file_data;
pub use file_data::*;
pub mod geometry;
mod global_attributes;
pub mod input_data;
#[cfg(feature = "native-bind")]
pub mod native_bind;
pub mod point_interaction;
mod render_template;
#[cfg(feature = "wasm-bind")]
mod web_sys_bind;

#[cfg(feature = "serialize")]
mod transit;

#[cfg(feature = "serialize")]
pub use transit::*;

pub use elements::*;
pub use events::*;
pub use global_attributes::*;
pub use render_template::*;

#[cfg(feature = "eval")]
pub mod eval;

pub mod extensions {
    pub use crate::elements::extensions::*;
    pub use crate::global_attributes::{GlobalAttributesExtension, SvgAttributesExtension};
}

pub mod prelude {
    pub use crate::elements::extensions::*;
    #[cfg(feature = "eval")]
    pub use crate::eval::*;
    pub use crate::events::*;
    pub use crate::global_attributes::{GlobalAttributesExtension, SvgAttributesExtension};
    pub use crate::point_interaction::*;
    pub use keyboard_types::{self, Code, Key, Location, Modifiers};
}
