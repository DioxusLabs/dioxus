#![allow(non_snake_case)]
#![doc = include_str!("../README.md")]

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
    pub(crate) use crate::diff::*;
    pub use crate::lazynodes::*;
    pub use crate::mutations::*;
    pub use crate::nodes::*;
    pub use crate::scope::*;
    pub(crate) use crate::scopearena::*;
    pub use crate::virtual_dom::*;

    pub type Element<'a> = Option<VNode<'a>>;
    pub type Component<P> = for<'a> fn(Scope<'a, P>) -> Element<'a>;
}

pub use crate::innerlude::{
    Attribute, Component, DioxusElement, DomEdit, Element, ElementId, EventHandler, EventPriority,
    IntoVNode, LazyNodes, Listener, Mutations, NodeFactory, Properties, SchedulerMsg, Scope,
    ScopeId, ScopeState, UserEvent, VElement, VFragment, VNode, VirtualDom,
};

pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties};
    pub use crate::innerlude::Scope;
    pub use crate::innerlude::{
        Component, DioxusElement, Element, EventHandler, LazyNodes, NodeFactory, ScopeState,
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
