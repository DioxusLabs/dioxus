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

pub mod elements;
#[cfg(feature = "hot-reload-context")]
pub use elements::HtmlCtx;
#[cfg(feature = "html-to-rsx")]
pub use elements::{map_html_attribute_to_rsx, map_html_element_to_rsx};
mod attribute_groups;
pub mod events;
mod render_template;

pub use bytes;

pub use html_events::*;

pub use attribute_groups::*;
pub use elements::*;
pub use events::*;
pub use render_template::*;

pub use crate::attribute_groups::{GlobalAttributesExtension, SvgAttributesExtension};
pub use crate::elements::extensions::*;
pub use keyboard_types::{self, Code, Key, Location, Modifiers};

pub mod extensions {
    pub use crate::attribute_groups::{GlobalAttributesExtension, SvgAttributesExtension};
    pub use crate::elements::extensions::*;
}
