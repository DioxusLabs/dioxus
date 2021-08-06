use std::cell::Cell;

use crate::innerlude::*;

// create a cell with a "none" value
#[inline]
pub fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

// /// A helper type that lets scopes be ordered by their height
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub struct HeightMarker {
//     pub idx: ScopeId,
//     pub height: u32,
// }

// impl Ord for HeightMarker {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         self.height.cmp(&other.height)
//     }
// }

// impl PartialOrd for HeightMarker {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.cmp(other))
//     }
// }

pub struct DebugDom {}
impl DebugDom {
    pub fn new() -> Self {
        Self {}
    }
}

impl RealDom for DebugDom {
    fn raw_node_as_any(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}
