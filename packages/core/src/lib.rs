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
pub(crate) mod component;
pub(crate) mod diff;
pub(crate) mod lazynodes;
pub(crate) mod mutations;
pub(crate) mod nodes;
pub(crate) mod scope;
pub(crate) mod scopearena;
pub(crate) mod virtual_dom;

pub(crate) mod innerlude {
    pub use crate::component::*;
    pub use crate::diff::*;
    pub use crate::lazynodes::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::scope::*;
    pub use crate::scopearena::*;
    pub use crate::virtual_dom::*;

    pub type Element = Option<VPortal>;
    pub type Component<P> = for<'a> fn(Context<'a>, &'a P) -> Element;
}

pub use crate::innerlude::{
    Attribute, Component, Context, DioxusElement, DomEdit, Element, ElementId, EventHandler,
    EventPriority, IntoVNode, LazyNodes, Listener, MountType, Mutations, NodeFactory, Properties,
    SchedulerMsg, ScopeId, UserEvent, VElement, VFragment, VNode, VirtualDom,
};

pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties};
    pub use crate::innerlude::Context;
    pub use crate::innerlude::{
        Component, DioxusElement, Element, EventHandler, LazyNodes, NodeFactory, Scope,
    };
    pub use crate::nodes::VNode;
    pub use crate::VirtualDom;
}

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    // the foundation of this library
    pub use bumpalo;
    pub use futures_channel;
}
