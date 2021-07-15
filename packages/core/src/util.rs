use std::{
    cell::{Cell, RefCell, RefMut},
    rc::Rc,
};

use futures_util::StreamExt;
use slotmap::{DefaultKey, Key, KeyData};

use crate::innerlude::*;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct EventQueue {
    pub queue: Rc<RefCell<Vec<HeightMarker>>>,
}

impl EventQueue {
    pub fn new_channel(&self, height: u32, idx: ScopeId) -> Rc<dyn Fn()> {
        let inner = self.clone();
        let marker = HeightMarker { height, idx };
        Rc::new(move || {
            log::debug!("channel updated {:#?}", marker);
            inner.queue.as_ref().borrow_mut().push(marker)
        })
    }

    pub fn sort_unstable(&self) {
        self.queue.borrow_mut().sort_unstable()
    }

    pub fn borrow_mut(&self) -> RefMut<Vec<HeightMarker>> {
        self.queue.borrow_mut()
    }
}

/// A helper type that lets scopes be ordered by their height
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeightMarker {
    pub idx: ScopeId,
    pub height: u32,
}

impl Ord for HeightMarker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl PartialOrd for HeightMarker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The `RealDomNode` is an ID handle that corresponds to a foreign DOM node.
///
/// "u64" was chosen for two reasons
/// - 0 cost hashing
/// - use with slotmap and other versioned slot arenas

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealDomNode(pub u64);
impl RealDomNode {
    #[inline]
    pub fn empty() -> Self {
        Self(u64::MIN)
    }
    #[inline]
    pub fn empty_cell() -> Cell<Self> {
        Cell::new(Self::empty())
    }
    #[inline]
    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct DebugDom {
    counter: u64,
}
impl DebugDom {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl<'a> RealDom<'a> for DebugDom {
    fn raw_node_as_any(&self) -> &mut dyn std::any::Any {
        todo!()
    }

    fn request_available_node(&mut self) -> RealDomNode {
        self.counter += 1;
        RealDomNode::from_u64(self.counter)
    }
}
