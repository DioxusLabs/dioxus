use dioxus_core::{ReactiveContext, SubscriberList, Subscribers};
use dioxus_signals::{CopyValue, ReadableExt, SyncStorage, Writable, WritableExt};
use std::fmt::Debug;
use std::hash::BuildHasher;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Default)]
pub(crate) struct SelectorNode {
    subscribers: HashSet<ReactiveContext>,
    root: HashMap<PathKey, SelectorNode>,
}

impl SelectorNode {
    fn find(&self, path: &[PathKey]) -> Option<&SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root.get(first).and_then(|child| child.find(rest))
    }

    fn find_mut(&mut self, path: &[PathKey]) -> Option<&mut SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root
            .get_mut(first)
            .and_then(|child| child.find_mut(rest))
    }

    fn get_mut_or_default(&mut self, path: &[PathKey    ]) -> &mut SelectorNode {
        let [first, rest @ ..] = path else {
            return self;
        };
        self.root
            .entry(*first)
            .or_default()
            .get_mut_or_default(rest)
    }

    fn visit_depth_first_mut(&mut self, f: &mut dyn FnMut(&mut SelectorNode)) {
        f(self);
        for child in self.root.values_mut() {
            child.visit_depth_first_mut(f);
        }
    }

    fn mark_children_dirty(&mut self, path: &[PathKey]) {
        let Some(node) = self.find_mut(path) else {
            return;
        };

        // Mark the node and all its children as dirty
        node.visit_depth_first_mut(&mut |node| {
            node.mark_dirty();
        });
    }

    fn mark_dirty_at_and_after_index(&mut self, path: &[PathKey], index: usize) {
        let Some(node) = self.find_mut(path) else {
            return;
        };

        // Mark the nodes before the index as dirty
        for (i, child) in node.root.iter_mut() {
            if *i as usize >= index {
                child.visit_depth_first_mut(&mut |node| {
                    node.mark_dirty();
                });
            }
        }
    }

    fn mark_dirty_shallow(&mut self, path: &[PathKey]) {
        let Some(node) = self.find_mut(path) else {
            return;
        };

        // Mark the node as dirty
        node.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        // We cannot hold the subscribers lock while calling mark_dirty, because mark_dirty can run user code which may cause a new subscriber to be added. If we hold the lock, we will deadlock.
        #[allow(clippy::mutable_key_type)]
        let mut subscribers = std::mem::take(&mut self.subscribers);
        subscribers.retain(|reactive_context| reactive_context.mark_dirty());
        // Extend the subscribers list instead of overwriting it in case a subscriber is added while reactive contexts are marked dirty
        self.subscribers.extend(subscribers);
    }

    fn remove(&mut self, path: &[PathKey]) {
        let [first, rest @ ..] = path else {
            return;
        };
        if let Some(node) = self.root.get_mut(first) {
            if rest.is_empty() {
                self.root.remove(first);
            } else {
                node.remove(rest);
            }
        }
    }
}

pub(crate) type PathKey = u16;
#[cfg(feature = "large-path")]
const PATH_LENGTH: usize = 32;
#[cfg(not(feature = "large-path"))]
const PATH_LENGTH: usize = 16;

#[derive(Copy, Clone, PartialEq)]
pub(crate) struct TinyVec {
    length: usize,
    path: [PathKey; PATH_LENGTH],
}

impl Default for TinyVec {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for TinyVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TinyVec")
            .field("path", &&self.path[..self.length])
            .finish()
    }
}

impl TinyVec {
    pub(crate) const fn new() -> Self {
        Self {
            length: 0,
            path: [0; PATH_LENGTH],
        }
    }

    pub(crate) const fn push(&mut self, index: u16) {
        if self.length < self.path.len() {
            self.path[self.length] = index;
            self.length += 1;
        } else {
            panic!("SelectorPath is full");
        }
    }
}

impl Deref for TinyVec {
    type Target = [u16];

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
pub(crate) struct StoreSubscriptions {
    inner: CopyValue<StoreSubscriptionsInner, SyncStorage>,
}

impl Clone for StoreSubscriptions {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for StoreSubscriptions {}

impl PartialEq for StoreSubscriptions {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl StoreSubscriptions {
    pub(crate) fn new() -> Self {
        Self {
            inner: CopyValue::new_maybe_sync(StoreSubscriptionsInner {
                root: SelectorNode::default(),
                hasher: std::collections::hash_map::RandomState::new(),
            }),
        }
    }

    pub(crate) fn hash(&self, index: impl Hash) -> PathKey {
        self.inner.write_unchecked().hasher.hash_one(&index) as PathKey
    }

    pub(crate) fn track(&self, key: &[PathKey]) {
        if let Some(rc) = ReactiveContext::current() {
            let subscribers = self.subscribers(key);
            rc.subscribe(subscribers);
        }
    }

    pub(crate) fn track_recursive(&self, key: &[PathKey]) {
        if let Some(rc) = ReactiveContext::current() {
            let mut paths = Vec::new();
            {
                let mut write = self.inner.write_unchecked();

                let root = write.root.get_mut_or_default(key);
                let mut nodes = vec![(key.to_vec(), &*root)];
                while let Some((path, node)) = nodes.pop() {
                    for (child_key, child_node) in &node.root {
                        let mut new_path = path.clone();
                        new_path.push(*child_key);
                        nodes.push((new_path, child_node));
                    }
                    paths.push(path);
                }
            }
            for path in paths {
                let subscribers = self.subscribers(&path);
                rc.subscribe(subscribers);
            }
        }
    }

    pub(crate) fn mark_dirty(&self, key: &[PathKey]) {
        self.inner.write_unchecked().root.mark_children_dirty(key);
    }

    pub(crate) fn mark_dirty_shallow(&self, key: &[PathKey]) {
        self.inner.write_unchecked().root.mark_dirty_shallow(key);
    }

    pub(crate) fn mark_dirty_at_and_after_index(&self, key: &[PathKey], index: usize) {
        self.inner
            .write_unchecked()
            .root
            .mark_dirty_at_and_after_index(key, index);
    }

    pub(crate) fn subscribers(&self, key: &[PathKey]) -> Subscribers {
        Arc::new(StoreSubscribers {
            subscriptions: *self,
            path: key.to_vec().into_boxed_slice(),
        })
        .into()
    }
}

struct StoreSubscribers {
    subscriptions: StoreSubscriptions,
    path: Box<[PathKey]>,
}

impl SubscriberList for StoreSubscribers {
    fn add(&self, subscriber: ReactiveContext) {
        let Ok(mut write) = self.subscriptions.inner.try_write_unchecked() else {
            return;
        };
        let node = write.root.get_mut_or_default(&self.path);
        node.subscribers.insert(subscriber);
    }

    fn remove(&self, subscriber: &ReactiveContext) {
        let Ok(mut write) = self.subscriptions.inner.try_write_unchecked() else {
            return;
        };
        let Some(node) = write.root.find_mut(&self.path) else {
            return;
        };
        node.subscribers.remove(subscriber);
        if node.subscribers.is_empty() && node.root.is_empty() {
            write.root.remove(&self.path);
        }
    }

    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext)) {
        let Ok(read) = self.subscriptions.inner.try_read() else {
            return;
        };
        let Some(node) = read.root.find(&self.path) else {
            return;
        };
        node.subscribers.iter().for_each(f);
    }
}
