#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]

use std::any::Any;
use std::hash::BuildHasherDefault;

use node_ref::NodeMask;
use rustc_hash::FxHasher;

pub mod custom_element;
#[cfg(feature = "dioxus")]
pub mod dioxus;
#[cfg(feature = "layout-attributes")]
pub mod layout_attributes;
pub mod node;
pub mod node_ref;
pub mod node_watcher;
mod passes;
pub mod real_dom;
pub mod tree;
pub mod utils;

pub use shipyard::EntityId as NodeId;

pub mod exports {
    //! Important dependencies that are used by the rest of the library
    //! Feel free to just add the dependencies in your own Crates.toml
    // exported for the macro
    #[doc(hidden)]
    pub use rustc_hash::FxHashSet;
    pub use shipyard;
}

/// A prelude of commonly used items
pub mod prelude {
    #[cfg(feature = "dioxus")]
    pub use crate::dioxus::*;
    pub use crate::node::{ElementNode, FromAnyValue, NodeType, OwnedAttributeView, TextNode};
    pub use crate::node_ref::{AttributeMaskBuilder, NodeMaskBuilder, NodeView};
    pub use crate::passes::{run_pass, PassDirection, RunPassView, TypeErasedState};
    pub use crate::passes::{Dependancy, DependancyView, Dependants, State};
    pub use crate::real_dom::{NodeImmutable, NodeMut, NodeRef, RealDom};
    pub use crate::NodeId;
    pub use crate::SendAnyMap;
}

/// A map that can be sent between threads
pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
/// A set that can be sent between threads
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;
/// A map of types that can be sent between threads
pub type SendAnyMap = anymap::Map<dyn Any + Send + Sync + 'static>;
