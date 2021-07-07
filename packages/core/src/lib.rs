//! Dioxus Core
//! ----------
//!
//!
//!
//!
//!
//!
//!

#[cfg(feature = "serialize")]
pub mod serialize;

pub mod arena;
pub mod component;
pub mod styles;
pub mod util; // Logic for extending FC

// pub mod debug_renderer;
pub mod diff;

pub mod error; // Error type we expose to the renderers
pub mod events; // Manages the synthetic event API
pub mod hooks; // Built-in hooks
pub mod nodebuilder; // Logic for building VNodes with a direct syntax
pub mod nodes; // Logic for the VNodes
pub mod signals;
pub mod virtual_dom; // Most fun logic starts here, manages the lifecycle and suspense

pub mod builder {
    pub use super::nodebuilder::*;
}

// types used internally that are important
pub(crate) mod innerlude {
    pub use crate::component::*;

    pub use crate::diff::*;
    pub use crate::error::*;
    pub use crate::events::*;
    pub use crate::hooks::*;
    pub use crate::nodebuilder::*;
    pub use crate::nodes::*;
    pub use crate::virtual_dom::*;

    pub type FC<P> = fn(Context<P>) -> VNode;

    // Re-export the FC macro
    pub use crate::nodebuilder as builder;
    pub use dioxus_core_macro::{html, rsx};
}

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties};
    use crate::nodes;
    pub use crate::styles::{AsAttr, StyleBuilder};
    pub use crate::virtual_dom::Context;
    pub use crate::virtual_dom::Scoped;
    pub use nodes::*;

    pub use crate::nodebuilder::LazyNodes;

    pub use crate::nodebuilder::{DioxusElement, NodeFactory};
    // pub use nodes::iterables::IterableNodes;
    /// This type alias is an internal way of abstracting over the static functions that represent components.
    pub use crate::innerlude::FC;

    // expose our bumpalo type
    pub use bumpalo;
    pub use bumpalo::Bump;

    // Re-export the FC macro
    pub use crate::nodebuilder as builder;
    // pub use dioxus_core_macro::fc;

    pub use dioxus_core_macro::{format_args_f, html, rsx, Props};

    pub use crate::diff::DiffMachine;
    pub use crate::virtual_dom::ScopeIdx;

    // pub use crate::debug_renderer::DebugRenderer;
    pub use crate::hooks::*;
}
