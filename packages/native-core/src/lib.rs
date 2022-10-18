use std::cmp::Ordering;

use dioxus_core::ElementId;

pub mod layout_attributes;
pub mod node_ref;
pub mod real_dom;
pub mod state;
#[doc(hidden)]
pub mod traversable;
pub mod utils;

/// A id for a node that lives in the real dom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealNodeId {
    ElementId(ElementId),
    UnaccessableId(usize),
}

impl RealNodeId {
    pub fn as_element_id(&self) -> ElementId {
        match self {
            RealNodeId::ElementId(id) => *id,
            RealNodeId::UnaccessableId(_) => panic!("Expected element id"),
        }
    }

    pub fn as_unaccessable_id(&self) -> usize {
        match self {
            RealNodeId::ElementId(_) => panic!("Expected unaccessable id"),
            RealNodeId::UnaccessableId(id) => *id,
        }
    }
}

impl Ord for RealNodeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for RealNodeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::ElementId(a), Self::ElementId(b)) => a.0.partial_cmp(&b.0),
            (Self::UnaccessableId(a), Self::UnaccessableId(b)) => a.partial_cmp(b),
            (Self::ElementId(_), Self::UnaccessableId(_)) => Some(Ordering::Greater),
            (Self::UnaccessableId(_), Self::ElementId(_)) => Some(Ordering::Less),
        }
    }
}

/// Used in derived state macros
#[derive(Eq, PartialEq)]
#[doc(hidden)]
pub struct HeightOrdering {
    pub height: u16,
    pub id: RealNodeId,
}

impl HeightOrdering {
    pub fn new(height: u16, id: RealNodeId) -> Self {
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
