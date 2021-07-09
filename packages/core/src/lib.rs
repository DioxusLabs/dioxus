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
pub mod bumpframe;
pub mod component;
pub mod context;
pub mod diff;
pub mod error;
pub mod events;
pub mod hooklist;
pub mod nodebuilder;
pub mod nodes;
pub mod scope;
pub mod signals;
pub mod styles;
pub mod tasks;
pub mod util;
pub mod virtual_dom;

pub mod builder {
    pub use super::nodebuilder::*;
}

// types used internally that are important
pub(crate) mod innerlude {
    pub use crate::bumpframe::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::diff::*;
    pub use crate::error::*;
    pub use crate::events::*;
    pub use crate::nodebuilder::*;
    pub use crate::nodes::*;
    pub use crate::scope::*;
    pub use crate::util::*;
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
    pub use crate::context::Context;
    use crate::nodes;
    pub use crate::styles::{AsAttr, StyleBuilder};

    pub use crate::util::RealDomNode;
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
}
