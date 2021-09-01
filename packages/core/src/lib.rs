#![allow(non_snake_case)]
#![doc = include_str!("../README.md")]

/*
Navigating this crate:
- virtual_dom: the primary entrypoint for the crate
- scheduler: the core interior logic called by virtual_dom
- nodes: the definition of VNodes, listeners, etc.
- diff: the stackmachine-based diffing algorithm
- hooks: foundational hooks that require crate-private APIs
- mutations: DomEdits/NodeRefs and internal API to create them

Some utilities
*/
pub mod bumpframe;
pub mod childiter;
pub mod component;
pub mod context;
pub mod diff;
pub mod diff_stack;
pub mod events;
pub mod heuristics;
pub mod hooklist;
pub mod hooks;
pub mod mutations;
pub mod nodes;
pub mod scheduler;
pub mod scope;
pub mod util;
pub mod virtual_dom;

pub(crate) mod innerlude {
    pub(crate) use crate::bumpframe::*;
    pub(crate) use crate::childiter::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub(crate) use crate::diff::*;
    pub use crate::diff_stack::*;
    pub use crate::events::*;
    pub use crate::heuristics::*;
    pub(crate) use crate::hooklist::*;
    pub use crate::hooks::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::scheduler::*;
    pub use crate::scope::*;
    pub use crate::util::*;
    pub use crate::virtual_dom::*;

    pub type DomTree<'a> = Option<VNode<'a>>;
    pub type FC<P> = fn(Context<P>) -> DomTree;

    pub use dioxus_core_macro::{format_args_f, html, rsx};
}

pub use crate::innerlude::{
    format_args_f, html, rsx, Context, DiffInstruction, DioxusElement, DomEdit, DomTree, ElementId,
    EventPriority, LazyNodes, MountType, Mutations, NodeFactory, Properties, ScopeId,
    SuspendedContext, SyntheticEvent, UiEvent, VNode, VirtualDom, FC,
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

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    // the foundation of this library
    pub use bumpalo;
}
