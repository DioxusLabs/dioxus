use dioxus_core::{ReactiveContext, SubscriberList, Subscribers};
use dioxus_signals::{CopyValue, ReadableExt, SyncStorage, Writable, WritableExt};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::BuildHasher;
use std::ops::BitOrAssign;
use std::{collections::HashMap, hash::Hash, ops::Deref, sync::Arc};

/// A single node in the [`StoreSubscriptions`] tree. Each path is a specific view into the store
/// and can be subscribed to and marked dirty separately.
#[derive(Clone, Default)]
pub(crate) struct SelectorNode {
    subscribers: HashMap<ReactiveContext, SubscriptionDepth>,
    root: HashMap<PathKey, SelectorNode>,
}

impl SelectorNode {
    /// Get an existing selector node by its path.
    fn get(&self, path: &[PathKey]) -> Option<&SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root.get(first).and_then(|child| child.get(rest))
    }

    /// Get an existing selector node by its path mutably.
    fn get_mut(&mut self, path: &[PathKey]) -> Option<&mut SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root
            .get_mut(first)
            .and_then(|child| child.get_mut(rest))
    }

    /// Get a selector mutably or create one if it doesn't exist. This is used when subscribing to
    /// a path that may not exist yet.
    fn get_mut_or_default(&mut self, path: &[PathKey]) -> &mut SelectorNode {
        let [first, rest @ ..] = path else {
            return self;
        };
        self.root
            .entry(*first)
            .or_default()
            .get_mut_or_default(rest)
    }

    /// Get the path to each node under this node
    ///
    /// This is used to mark nodes dirty recursively when a Store is written to.
    fn paths_under(&self, current_path: &[PathKey], paths: &mut Vec<Box<[PathKey]>>) {
        paths.push(current_path.into());
        for (i, child) in self.root.iter() {
            let mut child_path: Vec<PathKey> = current_path.into();
            child_path.push(*i);
            child.paths_under(&child_path, paths);
        }
    }

    /// Get paths to children at and after a certain index.
    ///
    /// This is used when inserting a new item into a list.
    /// Items after the index that is inserted need to be marked dirty because the value that index points to may have changed.
    fn paths_at_and_after_index(
        &self,
        path: &[PathKey],
        index: usize,
        paths: &mut Vec<Box<[PathKey]>>,
    ) {
        let Some(node) = self.get(path) else {
            return;
        };

        // Mark the nodes at and after the index as dirty
        for (i, child) in node.root.iter() {
            if *i as usize >= index {
                let mut child_path: Vec<PathKey> = path.into();
                child_path.push(*i);
                child.paths_under(&child_path, paths);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.subscribers.is_empty() && self.root.is_empty()
    }

    fn prune_if_empty(&mut self, path: &[PathKey]) {
        let [first, rest @ ..] = path else {
            return;
        };
        let Some(child) = self.root.get_mut(first) else {
            return;
        };
        child.prune_if_empty(rest);
        if child.is_empty() {
            self.root.remove(first);
        }
    }

    fn add_subscriber(&mut self, reactive_context: ReactiveContext, depth: SubscriptionDepth) {
        self.subscribers
            .entry(reactive_context)
            .and_modify(|existing_depth| {
                *existing_depth |= depth;
            })
            .or_insert(depth);
    }

    fn remove_subscriber(&mut self, reactive_context: &ReactiveContext) {
        self.subscribers.remove(reactive_context);
    }

    // ReactiveContext uses stable pointer/id-based Eq+Hash, so it is safe as a map key here.
    #[allow(clippy::mutable_key_type)]
    fn take_subscribers(&mut self) -> HashMap<ReactiveContext, SubscriptionDepth> {
        std::mem::take(&mut self.subscribers)
    }

    // ReactiveContext uses stable pointer/id-based Eq+Hash, so it is safe as a map key here.
    #[allow(clippy::mutable_key_type)]
    fn restore_subscribers(&mut self, subscribers: HashMap<ReactiveContext, SubscriptionDepth>) {
        self.subscribers.extend(subscribers);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubscriptionDepth {
    Shallow,
    Deep,
}

impl BitOrAssign for SubscriptionDepth {
    fn bitor_assign(&mut self, rhs: Self) {
        match (*self, rhs) {
            (SubscriptionDepth::Shallow, SubscriptionDepth::Shallow) => {}
            _ => *self = SubscriptionDepth::Deep,
        }
    }
}

impl SubscriptionDepth {
    fn includes_deep(self) -> bool {
        matches!(self, Self::Deep)
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

    pub(crate) fn from_slice(path: &[PathKey]) -> Self {
        let mut out = Self::new();
        for key in path {
            out.push(*key);
        }
        out
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
    /// Create a new instance of StoreSubscriptions.
    pub(crate) fn new() -> Self {
        Self {
            inner: CopyValue::new_maybe_sync(StoreSubscriptionsInner {
                root: SelectorNode::default(),
                hasher: std::collections::hash_map::RandomState::new(),
            }),
        }
    }

    /// Hash an index into a PathKey using the hasher. The hash should be consistent
    /// across calls
    pub(crate) fn hash(&self, index: &impl Hash) -> PathKey {
        (self.inner.write_unchecked().hasher.hash_one(index) % PathKey::MAX as u64) as PathKey
    }

    /// Subscribe shallowly to a specific path in the store.
    ///
    /// Shallow subscriptions rerun only when this exact path is written to.
    pub(crate) fn track(&self, key: &[PathKey]) {
        if let Some(rc) = ReactiveContext::current() {
            let subscribers = self.shallow_subscribers(key);
            rc.subscribe(subscribers);
        }
    }

    /// Subscribe deeply to a specific path in the store.
    ///
    /// Deep subscriptions rerun when this path or any descendant path is written to.
    pub(crate) fn track_deep(&self, key: &[PathKey]) {
        if let Some(rc) = ReactiveContext::current() {
            let subscribers = self.deep_subscribers(key);
            rc.subscribe(subscribers);
        }
    }

    /// Mark the written node and its descendants as dirty.
    ///
    /// Deep subscribers on ancestor nodes are also notified because child writes
    /// change the value observed by a deep read of the parent.
    pub(crate) fn mark_dirty(&self, key: &[PathKey]) {
        self.mark_node_dirty(key);
        let descendant_paths = {
            let read = &self.inner.read_unchecked();
            let mut paths = Vec::new();
            if let Some(node) = read.root.get(key) {
                for (child_key, child_node) in &node.root {
                    let mut child_path: Vec<PathKey> = key.into();
                    child_path.push(*child_key);
                    child_node.paths_under(&child_path, &mut paths);
                }
            }
            paths
        };
        for path in descendant_paths {
            self.mark_node_subscribers_dirty(&path);
        }
    }

    /// Mark all subscribers on a single node as dirty.
    ///
    /// Deep subscribers on ancestors are also notified because an exact write to
    /// this path changes any deep read of its parents.
    pub(crate) fn mark_node_dirty(&self, key: &[PathKey]) {
        for i in 0..key.len() {
            self.mark_ancestor_deep_subscribers_dirty(&key[..i]);
        }
        self.mark_node_subscribers_dirty(key);
    }

    /// Mark only deep subscribers for a single ancestor node as dirty.
    fn mark_ancestor_deep_subscribers_dirty(&self, key: &[PathKey]) {
        self.retain_subscribers(key, |reactive_context, depth| {
            !depth.includes_deep() || reactive_context.mark_dirty()
        });
    }

    /// Mark all nodes at and after the index and their children as dirty.
    pub(crate) fn mark_dirty_at_and_after_index(&self, key: &[PathKey], index: usize) {
        let paths = {
            let read = self.inner.read_unchecked();
            let mut paths = Vec::new();
            read.root.paths_at_and_after_index(key, index, &mut paths);
            paths
        };
        for path in paths {
            self.mark_node_dirty(&path);
        }
    }

    /// Get the shallow subscribers for a specific path in the store.
    pub(crate) fn shallow_subscribers(&self, key: &[PathKey]) -> Subscribers {
        Arc::new(StoreSubscribers {
            subscriptions: *self,
            path: TinyVec::from_slice(key),
            depth: SubscriptionDepth::Shallow,
        })
        .into()
    }

    /// Get the deep subscribers for a specific path in the store.
    pub(crate) fn deep_subscribers(&self, key: &[PathKey]) -> Subscribers {
        Arc::new(StoreSubscribers {
            subscriptions: *self,
            path: TinyVec::from_slice(key),
            depth: SubscriptionDepth::Deep,
        })
        .into()
    }

    fn retain_subscribers(
        &self,
        key: &[PathKey],
        mut retain: impl FnMut(&ReactiveContext, SubscriptionDepth) -> bool,
    ) {
        // We cannot hold the subscribers lock while calling mark_dirty, because
        // mark_dirty can run user code which may cause a new subscriber to be
        // added. If we hold the lock, we will deadlock.
        // ReactiveContext uses stable pointer/id-based Eq+Hash, so it is safe as a map key here.
        #[allow(clippy::mutable_key_type)]
        let mut subscribers = {
            let mut write = self.inner.write_unchecked();
            let Some(node) = write.root.get_mut(key) else {
                return;
            };
            node.take_subscribers()
        };
        subscribers.retain(|reactive_context, depth| retain(reactive_context, *depth));

        // Extend the subscribers list instead of overwriting it in case a
        // subscriber is added while reactive contexts are marked dirty.
        let mut write = self.inner.write_unchecked();
        let Some(node) = write.root.get_mut(key) else {
            return;
        };
        node.restore_subscribers(subscribers);
    }

    fn mark_node_subscribers_dirty(&self, key: &[PathKey]) {
        self.retain_subscribers(key, |reactive_context, _| reactive_context.mark_dirty());
    }
}

/// A subscriber list implementation that handles garbage collection of the subscription tree.
struct StoreSubscribers {
    subscriptions: StoreSubscriptions,
    path: TinyVec,
    depth: SubscriptionDepth,
}

impl StoreSubscribers {
    fn visible_prefixes(&self) -> impl Iterator<Item = &[PathKey]> {
        (0..=self.path.len())
            .rev()
            .map(move |len| &self.path[..len])
    }
}

impl SubscriberList for StoreSubscribers {
    /// Add a subscriber to the subscription list for this path in the store, creating the node if it doesn't exist.
    fn add(&self, subscriber: ReactiveContext) {
        let Ok(mut write) = self.subscriptions.inner.try_write_unchecked() else {
            return;
        };
        let node = write.root.get_mut_or_default(&self.path);
        node.add_subscriber(subscriber, self.depth);
    }

    /// Remove a subscriber from the subscription list for this path in the store. If the node has no subscribers left
    /// remove that node from the subscription tree.
    fn remove(&self, subscriber: &ReactiveContext) {
        let Ok(mut write) = self.subscriptions.inner.try_write_unchecked() else {
            return;
        };
        let mut empty_prefixes = Vec::new();
        for prefix in self.visible_prefixes() {
            let Some(node) = write.root.get_mut(prefix) else {
                continue;
            };
            let Some(depth) = node.subscribers.get(subscriber).copied() else {
                continue;
            };
            if prefix.len() == self.path.len() || depth.includes_deep() {
                node.remove_subscriber(subscriber);
                if node.is_empty() {
                    empty_prefixes.push(prefix.to_vec());
                }
            }
        }
        for prefix in empty_prefixes {
            write.root.prune_if_empty(&prefix);
        }
    }

    /// Visit all subscribers for this path in the store, calling the provided function on each subscriber.
    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext)) {
        let Ok(read) = self.subscriptions.inner.try_read() else {
            return;
        };
        // ReactiveContext uses stable pointer/id-based Eq+Hash, so it is safe in this set.
        #[allow(clippy::mutable_key_type)]
        let mut seen = HashSet::new();
        for prefix in self.visible_prefixes() {
            let Some(node) = read.root.get(prefix) else {
                continue;
            };
            for (reactive_context, depth) in &node.subscribers {
                if (prefix.len() == self.path.len() || depth.includes_deep())
                    && seen.insert(*reactive_context)
                {
                    f(reactive_context);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_core::{ScopeId, VNode, VirtualDom};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn empty_app() -> dioxus_core::Element {
        VNode::empty()
    }

    #[track_caller]
    fn counting_context(counter: Arc<AtomicUsize>) -> ReactiveContext {
        ReactiveContext::new_with_callback(
            move || {
                counter.fetch_add(1, Ordering::Relaxed);
            },
            ScopeId::ROOT,
            std::panic::Location::caller(),
        )
    }

    #[test]
    fn mark_dirty_marks_descendants_without_remarking_ancestors() {
        let dom = VirtualDom::new(empty_app);

        dom.in_scope(ScopeId::ROOT, || {
            let subscriptions = StoreSubscriptions::new();

            let root_count = Arc::new(AtomicUsize::new(0));
            let root = counting_context(root_count.clone());
            root.subscribe(subscriptions.deep_subscribers(&[]));

            let child_count = Arc::new(AtomicUsize::new(0));
            let child = counting_context(child_count.clone());
            child.subscribe(subscriptions.deep_subscribers(&[1, 2]));

            let leaf_count = Arc::new(AtomicUsize::new(0));
            let leaf = counting_context(leaf_count.clone());
            leaf.subscribe(subscriptions.shallow_subscribers(&[1, 2, 3]));

            subscriptions.mark_dirty(&[1]);

            assert_eq!(root_count.load(Ordering::Relaxed), 1);
            assert_eq!(child_count.load(Ordering::Relaxed), 1);
            assert_eq!(leaf_count.load(Ordering::Relaxed), 1);
        });
    }
}
