use crate::SelectorStorage;
use dioxus_core::prelude::ReactiveContext;
use dioxus_signals::{CopyValue, ReadableExt, Subscribers, UnsyncStorage, Writable};
use std::hash::{BuildHasher, Hasher};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
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

    fn get_mut_or_default(&mut self, path: &[u32]) -> &mut SelectorNode {
        let [first, rest @ ..] = path else {
            return self;
        };
        self.root
            .entry(*first)
            .or_default()
            .get_mut_or_default(rest)
    }

    fn read(&mut self, path: &[u32]) {
        let node = self.get_mut_or_default(path);
        node.track();
    }

    fn track(&mut self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers.clone());
        }
    }

    fn read_nested(&mut self, path: &[u32]) {
        let node = self.get_mut_or_default(path);
        node.visit_depth_first_mut(&mut |n| {
            n.track();
        });
    }

    fn visit_depth_first(&self, f: &mut dyn FnMut(&SelectorNode)) {
        f(self);
        for child in self.root.values() {
            child.visit_depth_first(f);
        }
    }

    fn visit_depth_first_mut(&mut self, f: &mut dyn FnMut(&mut SelectorNode)) {
        f(self);
        for child in self.root.values_mut() {
            child.visit_depth_first_mut(f);
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
pub(crate) struct StoreSubscriptionsInner {
    root: SelectorNode,
    hasher: std::collections::hash_map::RandomState,
}

#[derive(Default)]
pub(crate) struct StoreSubscriptions<S: SelectorStorage = UnsyncStorage> {
    inner: CopyValue<StoreSubscriptionsInner, S>,
}

impl<S: SelectorStorage> Clone for StoreSubscriptions<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: SelectorStorage> Copy for StoreSubscriptions<S> {}

impl<S: SelectorStorage> PartialEq for StoreSubscriptions<S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<S: SelectorStorage> StoreSubscriptions<S> {
    pub(crate) fn new() -> Self {
        Self {
            inner: CopyValue::new_maybe_sync(StoreSubscriptionsInner {
                root: SelectorNode::default(),
                hasher: std::collections::hash_map::RandomState::new(),
            }),
        }
    }

    pub(crate) fn hash(&self, index: impl Hash) -> u32 {
        let mut hasher = self.inner.write_unchecked().hasher.build_hasher();
        index.hash(&mut hasher);
        hasher.finish() as u32
    }

    pub(crate) fn track(&self, key: &[u32]) {
        self.inner.write_unchecked().root.read(key);
    }

    pub(crate) fn track_nested(&self, key: &[u32]) {
        self.inner.write_unchecked().root.read_nested(key);
    }

    pub(crate) fn mark_dirty(&self, key: &[u32]) {
        self.inner.read().root.mark_children_dirty(key);
    }

    pub(crate) fn mark_dirty_shallow(&self, key: &[u32]) {
        self.inner.read().root.mark_dirty_shallow(key);
    }

    pub(crate) fn mark_dirty_at_and_after_index(&self, key: &[u32], index: usize) {
        self.inner
            .read()
            .root
            .mark_dirty_at_and_after_index(key, index);
    }

    pub(crate) fn subscribers(&self, key: &[u32]) -> Option<Subscribers> {
        let read = self.inner.read();
        let node = read.root.find(key)?;
        Some(node.subscribers.clone())
    }
}
