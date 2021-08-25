#![allow(non_snake_case)]
#![doc = include_str!("../README.md")]
//! Dioxus Core
//! ----------
//!
//!
//!
//!
//!
//!
//!

pub use crate::innerlude::{
    format_args_f, html, rsx, Context, DiffInstruction, DioxusElement, DomEdit, DomTree, ElementId,
    EventPriority, EventTrigger, LazyNodes, MountType, Mutations, NodeFactory, Properties, ScopeId,
    SuspendedContext, SyntheticEvent, VNode, VirtualDom, FC,
};

pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties};
    pub use crate::context::Context;
    pub use crate::hooks::*;
    pub use crate::innerlude::{DioxusElement, DomTree, LazyNodes, Mutations, NodeFactory, FC};
    pub use crate::nodes::VNode;
    pub use crate::VirtualDom;
    pub use dioxus_core_macro::{format_args_f, html, rsx, Props};
}

// types used internally that are important
pub(crate) mod innerlude {
    pub use crate::bumpframe::*;
    pub use crate::childiter::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::diff::*;
    pub use crate::diff_stack::*;
    pub use crate::dom_edits::*;
    pub use crate::error::*;
    pub use crate::events::*;
    pub use crate::heuristics::*;
    pub use crate::hooklist::*;
    pub use crate::hooks::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::scheduler::*;
    // pub use crate::scheduler::*;
    pub use crate::scope::*;
    pub use crate::util::*;
    pub use crate::virtual_dom::*;
    pub use crate::yield_now::*;

    pub type DomTree<'a> = Option<VNode<'a>>;
    pub type FC<P> = fn(Context<P>) -> DomTree;

    pub use dioxus_core_macro::{format_args_f, html, rsx};
}

pub mod exports {
    //! Important dependencies that are used by the rest of the library

    // the foundation of this library
    pub use bumpalo;
}

pub mod bumpframe;
pub mod childiter;
pub mod component;
pub mod context;
pub mod diff;
pub mod diff_stack;
pub mod dom_edits;
pub mod error;
pub mod events;
pub mod heuristics;
pub mod hooklist;
pub mod hooks;
pub mod mutations;
pub mod nodes;
pub mod scheduler;
pub mod scope;
pub mod signals;
pub mod util;
pub mod virtual_dom;
pub mod yield_now;
