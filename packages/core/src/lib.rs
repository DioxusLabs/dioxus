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
pub mod nodes;
pub mod scope;
pub mod signals;
pub mod tasks;
pub mod util;
pub mod virtual_dom;

// types used internally that are important
pub(crate) mod innerlude {
    pub use crate::bumpframe::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::diff::*;
    pub use crate::error::*;
    pub use crate::events::*;
    pub use crate::nodes::*;
    pub use crate::scope::*;
    pub use crate::serialize::*;
    pub use crate::tasks::*;
    pub use crate::util::*;
    pub use crate::virtual_dom::*;

    pub type FC<P> = fn(Context<P>) -> VNode;

    pub use dioxus_core_macro::{html, rsx};
}

pub use crate::{
    innerlude::{
        DioxusElement, DomEdit, LazyNodes, NodeFactory, RealDom, RealDomNode, ScopeIdx, FC,
    },
    virtual_dom::VirtualDom,
};

pub mod prelude {
    pub use crate::component::{fc_to_builder, Fragment, Properties};
    pub use crate::context::Context;
    pub use crate::innerlude::DioxusElement;
    pub use crate::innerlude::{LazyNodes, NodeFactory, FC};
    pub use crate::nodes::VNode;
    pub use crate::VirtualDom;
    pub use dioxus_core_macro::{format_args_f, html, rsx, Props};
}

pub mod exports {
    // export important things here
}
