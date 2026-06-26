#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

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

/// The shared root every element associated-const hangs off of. Both the built-in
/// `define_elements!` invocation and user `define_elements!` invocations `impl` their
/// per-element traits for this type, so `html::div`, `html::main`, and user-defined
/// `html::mycustom` all live under one namespace and aggregate in `html::` autocomplete.
#[allow(non_camel_case_types)]
pub enum html {}

#[macro_use]
pub mod elements;
#[cfg(feature = "hot-reload-context")]
pub use elements::HtmlCtx;
#[cfg(feature = "html-to-rsx")]
pub use elements::{map_html_attribute_to_rsx, map_html_element_to_rsx};
pub mod events;
pub(crate) mod file_data;
pub use file_data::*;
mod attribute_groups;
mod data_transfer;
pub mod geometry;
pub mod input_data;
pub mod point_interaction;
pub use data_transfer::*;

pub use bytes;

#[doc(hidden)]
pub use dioxus_core;
#[doc(hidden)]
pub use dioxus_html_internal_macro::define_elements as __define_elements;

/// Define typed custom elements (and their typed attributes) for use in `rsx!`.
///
/// Reach for `define_elements!` when your app needs project-specific element
/// names or attributes that should type-check just like the built-in Dioxus
/// HTML elements. Each element becomes a typed builder, and each listed
/// attribute becomes a typed method you can set in `rsx!`.
///
/// Element and attribute identifiers are written in Rust (`snake_case` or
/// `camelCase`). Use `#[element(name = "...")]` / `#[attr(name = "...")]` to
/// control the rendered HTML name when it differs from the Rust identifier -
/// for example to emit a hyphenated custom-element tag or a `data-*` attribute.
///
/// # Example
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// dioxus::html::define_elements! {
///     // Renders as `<analytics-panel>` even though the Rust name is `analyticsPanel`.
///     #[element(name = "analytics-panel")]
///     analyticsPanel {
///         metric,
///         // Renders as the `data-region` attribute.
///         #[attr(name = "data-region")]
///         region,
///     }
/// }
///
/// fn app() -> Element {
///     rsx! {
///         analyticsPanel {
///             metric: "conversion-rate",
///             region: "north-america",
///             "Revenue dashboard"
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_elements {
    ($($tokens:tt)*) => {
        $crate::__dioxus_html_define_elements_with_detected_gated_attributes! {
            $crate::__define_elements,
            core = $crate::dioxus_core,
            html = $crate;
            $($tokens)*
        }
    };
}

#[cfg(feature = "serialize")]
mod transit;

#[cfg(feature = "serialize")]
pub use transit::*;

pub use crate::attribute_groups::{GlobalAttributesExtension, SvgAttributesExtension};
pub use crate::point_interaction::*;
pub use attribute_groups::*;
pub use elements::*;
pub use events::*;
pub use keyboard_types::{self, Code, Key, Location, Modifiers};

pub mod traits {
    pub use crate::events::*;
    pub use crate::point_interaction::*;
}

pub mod extensions {
    pub use crate::attribute_groups::*;
    pub use crate::elements::extensions::*;
    pub use crate::events::EventsExtension;
}
