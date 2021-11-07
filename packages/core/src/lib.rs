#![allow(non_snake_case)]
#![doc = include_str!("../README.md")]

/*
Navigating this crate:
- virtual_dom: the primary entrypoint for the crate
- scheduler: the core interior logic called by the [`VirtualDom`]
- nodes: the definition of VNodes, listeners, etc.
- diff: the stackmachine-based diffing algorithm
- hooks: foundational hooks that require crate-private APIs
- mutations: DomEdits/NodeRefs and internal API to create them

Some utilities
*/
pub(crate) mod bumpframe;
pub(crate) mod component;
pub(crate) mod diff;
pub(crate) mod diff_stack;
pub(crate) mod hooklist;
pub(crate) mod hooks;
pub(crate) mod lazynodes;
pub(crate) mod mutations;
pub(crate) mod nodes;
pub(crate) mod scope;
pub(crate) mod scopearena;
pub(crate) mod test_dom;
pub(crate) mod util;
pub(crate) mod virtual_dom;

#[cfg(feature = "debug_vdom")]
pub mod debug_dom;

pub(crate) mod innerlude {
    pub(crate) use crate::bumpframe::*;
    pub use crate::component::*;
    pub(crate) use crate::diff::*;
    pub use crate::diff_stack::*;
    pub(crate) use crate::hooklist::*;
    pub use crate::hooks::*;
    pub use crate::lazynodes::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::scope::*;
    pub use crate::scopearena::*;
    pub use crate::test_dom::*;
    pub use crate::util::*;
    pub use crate::virtual_dom::*;

    pub type Element = Option<NodeLink>;
    pub type FC<P> = for<'a> fn(Scope<'a, P>) -> Element;
}

pub use crate::innerlude::{
    Attribute, Context, DioxusElement, DomEdit, Element, ElementId, EventPriority, LazyNodes,
    Listener, MountType, Mutations, NodeFactory, Properties, SchedulerMsg, ScopeChildren, ScopeId,
    TestDom, UserEvent, VAnchor, VElement, VFragment, VNode, VSuspended, VirtualDom, FC,
};

pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties, Scope};
    pub use crate::hooks::*;
    pub use crate::innerlude::Context;
    pub use crate::innerlude::{DioxusElement, Element, LazyNodes, NodeFactory, ScopeChildren, FC};
    pub use crate::nodes::VNode;
    pub use crate::VirtualDom;
}

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    // the foundation of this library
    pub use bumpalo;
    pub use futures_channel;
}
