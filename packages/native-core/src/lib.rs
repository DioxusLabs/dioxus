use std::any::Any;
use std::hash::BuildHasherDefault;

pub use node_ref::NodeMask;
pub use passes::AnyMapLike;
pub use passes::{Dependancy, Pass, TypeErasedPass};
use rustc_hash::FxHasher;
pub use tree::NodeId;

pub mod layout_attributes;
pub mod node;
pub mod node_ref;
mod passes;
pub mod real_dom;
pub mod tree;
pub mod utils;

pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;
pub type SendAnyMap = anymap::Map<dyn Any + Send + Sync + 'static>;

/// Used in derived state macros
#[derive(Eq, PartialEq)]
#[doc(hidden)]
pub struct HeightOrdering {
    pub height: u16,
    pub id: NodeId,
}

impl HeightOrdering {
    pub fn new(height: u16, id: NodeId) -> Self {
        HeightOrdering { height, id }
    }
}

// not the ordering after height is just for deduplication it can be any ordering as long as it is consistent
impl Ord for HeightOrdering {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height).then(self.id.cmp(&other.id))
    }
}

impl PartialOrd for HeightOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
