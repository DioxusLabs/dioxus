use std::any::Any;
use std::hash::BuildHasherDefault;

pub use node_ref::NodeMask;
pub use passes::AnyMapLike;
pub use passes::{Dependancy, State, TypeErasedPass};
pub use real_dom::{NodeMut, NodeRef, RealDom};
use rustc_hash::FxHasher;

#[cfg(feature = "dioxus")]
pub mod dioxus;
pub mod layout_attributes;
pub mod node;
pub mod node_ref;
pub mod node_watcher;
mod passes;
pub mod real_dom;
pub mod tree;
pub mod utils;
pub use passes::run_pass;
pub use shipyard::EntityId as NodeId;

pub mod prelude {
    pub use crate::node::{ElementNode, FromAnyValue, NodeType, OwnedAttributeView, TextNode};
    pub use crate::node_ref::{AttributeMaskBuilder, NodeMaskBuilder, NodeView};
    pub use crate::passes::{Dependancy, State};
    pub use crate::real_dom::{NodeImmutable, NodeMut, NodeRef, RealDom};
    pub use crate::NodeId;
    pub use crate::SendAnyMap;
}

pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;
pub type SendAnyMap = anymap::Map<dyn Any + Send + Sync + 'static>;
