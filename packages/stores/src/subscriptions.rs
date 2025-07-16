use crate::SelectorStorage;
use dioxus_core::prelude::ReactiveContext;
use dioxus_signals::{CopyValue, ReadableExt, Subscribers, UnsyncStorage, Writable};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub(crate) struct SelectorNode {
    subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
    root: HashMap<u32, SelectorNode>,
}

impl SelectorNode {
    fn find(&self, path: &[u32]) -> Option<&SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root.get(first).and_then(|child| child.find(rest))
    }

    fn read(&mut self, path: &[u32]) {
        let [first, rest @ ..] = path else {
            if let Some(rc) = ReactiveContext::current() {
                rc.subscribe(self.subscribers.clone());
            }
            return;
        };
        self.root.entry(*first).or_default().read(rest);
    }

    fn visit_depth_first(&self, f: &mut dyn FnMut(&SelectorNode)) {
        for child in self.root.values() {
            child.visit_depth_first(f);
        }
    }

    fn mark_children_dirty(&self, path: &[u32]) {
        let Some(node) = self.find(path) else {
            return;
        };

        // Mark the node and all its children as dirty
        node.visit_depth_first(&mut |node| {
            node.mark_dirty();
        });
    }

    fn mark_dirty_at_and_after_index(&self, path: &[u32], index: usize) {
        let Some(node) = self.find(path) else {
            return;
        };

        // Mark the nodes before the index as dirty
        for (i, child) in node.root.iter() {
            if *i as usize >= index {
                child.visit_depth_first(&mut |node| {
                    node.mark_dirty();
                });
            }
        }
    }

    fn mark_dirty_shallow(&self, path: &[u32]) {
        let Some(node) = self.find(path) else {
            return;
        };

        // Mark the node as dirty
        node.mark_dirty();
    }

    fn mark_dirty(&self) {
        // We cannot hold the subscribers lock while calling mark_dirty, because mark_dirty can run user code which may cause a new subscriber to be added. If we hold the lock, we will deadlock.
        #[allow(clippy::mutable_key_type)]
        let mut subscribers = std::mem::take(&mut *self.subscribers.lock().unwrap());
        subscribers.retain(|reactive_context| reactive_context.mark_dirty());
        // Extend the subscribers list instead of overwriting it in case a subscriber is added while reactive contexts are marked dirty
        self.subscribers.lock().unwrap().extend(subscribers);
    }
}

#[derive(Copy, Clone, PartialEq)]
pub(crate) struct TinyVec {
    length: usize,
    path: [u32; 64],
}

impl Default for TinyVec {
    fn default() -> Self {
        Self::new()
    }
}

impl TinyVec {
    pub(crate) const fn new() -> Self {
        Self {
            length: 0,
            path: [0; 64],
        }
    }

    pub(crate) const fn push(&mut self, index: u32) {
        if self.length < self.path.len() {
            self.path[self.length] = index;
            self.length += 1;
        } else {
            panic!("SelectorPath is full");
        }
    }
}

impl Deref for TinyVec {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        &self.path[..self.length]
    }
}

#[derive(Default)]
pub(crate) struct StoreSubscriptions<S: SelectorStorage = UnsyncStorage> {
    root: CopyValue<SelectorNode, S>,
}

impl<S: SelectorStorage> Clone for StoreSubscriptions<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: SelectorStorage> Copy for StoreSubscriptions<S> {}

impl<S: SelectorStorage> PartialEq for StoreSubscriptions<S> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<S: SelectorStorage> StoreSubscriptions<S> {
    pub(crate) fn new() -> Self {
        Self {
            root: CopyValue::new_maybe_sync(SelectorNode::default()),
        }
    }

    pub(crate) fn track(&self, key: &[u32]) {
        self.root.write_unchecked().read(key);
    }

    pub(crate) fn mark_dirty(&self, key: &[u32]) {
        self.root.read().mark_children_dirty(key);
    }

    pub(crate) fn mark_dirty_shallow(&self, key: &[u32]) {
        self.root.read().mark_dirty_shallow(key);
    }

    pub(crate) fn mark_dirty_at_and_after_index(&self, key: &[u32], index: usize) {
        self.root.read().mark_dirty_at_and_after_index(key, index);
    }

    pub(crate) fn subscribers(&self, key: &[u32]) -> Option<Subscribers> {
        let read = self.root.read();
        let node = read.find(key)?;
        Some(node.subscribers.clone())
    }
}
