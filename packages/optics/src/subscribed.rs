//! Path-granular subscription for any optic chain.
//!
//! [`Subscribed`] wraps any accessor that implements
//! [`Pathed`](crate::Pathed) and [`Access`](crate::Access) (plus, optionally,
//! [`AccessMut`](crate::AccessMut)) and adds per-path subscription
//! bookkeeping *around* it. Plain signals and roots don't pay for this —
//! only `Subscribed` touches path machinery.
//!
//! The bookkeeping tree is identical in spirit to the one used by
//! `dioxus_stores::Store` (it supports shallow + deep subscribers, prunes
//! empty nodes, and notifies ancestors on descendant writes) but lives here
//! so it composes at any point in an optic chain — not just at the signal
//! root.

use std::{
    collections::{HashMap, HashSet},
    ops::BitOrAssign,
    sync::{Arc, Mutex},
};

use dioxus_core::{ReactiveContext, SubscriberList};
use generational_box::{AnyStorage, WriteLock};

use crate::combinator::{Access, AccessMut, ValueAccess};
use crate::path::{PathBuffer, PathSegment, Pathed};

/// Tree node used for path-granular subscription tracking.
#[derive(Default, Clone)]
struct Node {
    subscribers: HashMap<ReactiveContext, Depth>,
    children: HashMap<PathSegment, Node>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Depth {
    /// Subscriber only cares about exact-path writes.
    Shallow,
    /// Subscriber also cares about descendant writes.
    Deep,
}

impl BitOrAssign for Depth {
    fn bitor_assign(&mut self, rhs: Self) {
        match (*self, rhs) {
            (Depth::Shallow, Depth::Shallow) => {}
            _ => *self = Depth::Deep,
        }
    }
}

impl Depth {
    fn is_deep(self) -> bool {
        matches!(self, Depth::Deep)
    }
}

impl Node {
    fn get(&self, path: &[PathSegment]) -> Option<&Node> {
        match path {
            [] => Some(self),
            [first, rest @ ..] => self.children.get(first).and_then(|n| n.get(rest)),
        }
    }

    fn get_mut_or_default(&mut self, path: &[PathSegment]) -> &mut Node {
        match path {
            [] => self,
            [first, rest @ ..] => self
                .children
                .entry(*first)
                .or_default()
                .get_mut_or_default(rest),
        }
    }

    fn get_mut(&mut self, path: &[PathSegment]) -> Option<&mut Node> {
        match path {
            [] => Some(self),
            [first, rest @ ..] => self
                .children
                .get_mut(first)
                .and_then(|n| n.get_mut(rest)),
        }
    }

    fn is_empty(&self) -> bool {
        self.subscribers.is_empty() && self.children.is_empty()
    }

    fn prune(&mut self, path: &[PathSegment]) {
        match path {
            [] => {}
            [first, rest @ ..] => {
                if let Some(child) = self.children.get_mut(first) {
                    child.prune(rest);
                    if child.is_empty() {
                        self.children.remove(first);
                    }
                }
            }
        }
    }

    fn descendant_paths(
        &self,
        current: &[PathSegment],
        out: &mut Vec<Vec<PathSegment>>,
    ) {
        for (seg, child) in &self.children {
            let mut p = current.to_vec();
            p.push(*seg);
            out.push(p.clone());
            child.descendant_paths(&p, out);
        }
    }
}

#[derive(Default, Clone)]
struct TreeInner {
    root: Node,
}

/// Shared subscription tree — cheaply cloneable.
#[derive(Default, Clone)]
pub struct SubscriptionTree {
    inner: Arc<Mutex<TreeInner>>,
}

impl SubscriptionTree {
    /// Create an empty subscription tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe the current [`ReactiveContext`] (if any) shallowly at `path`.
    ///
    /// Shallow subscribers only fire on an exact-path write.
    pub fn track(&self, path: &[PathSegment]) {
        self.subscribe(path, Depth::Shallow);
    }

    /// Subscribe the current [`ReactiveContext`] (if any) deeply at `path`.
    ///
    /// Deep subscribers fire on an exact-path write *or* on a descendant write.
    pub fn track_deep(&self, path: &[PathSegment]) {
        self.subscribe(path, Depth::Deep);
    }

    /// Mark `path` (and all subscribers that care about it) dirty.
    pub fn notify(&self, path: &[PathSegment]) {
        self.mark_dirty(path);
    }

    /// Mark only the node at `path` (and its deep ancestors) dirty — not
    /// descendants. Matches the stores `mark_node_dirty` behavior: useful
    /// when the write only changed the node's own value, not any of its
    /// children.
    pub fn notify_node(&self, path: &[PathSegment]) {
        for i in 0..path.len() {
            self.retain_at(&path[..i], |_, depth| !depth.is_deep());
        }
        self.retain_at(path, |_, _| false);
    }

    /// Mark dirty every child of `path` whose segment is numerically `>= cutoff`.
    ///
    /// This supports insertion into ordered containers: inserting at position
    /// `i` shifts items `i..len`, so their subscriptions must rerun. Children
    /// whose segment wasn't constructed via [`PathSegment::index`] are also
    /// compared by their raw `u64` — hashed keys get mixed into the
    /// comparison but will virtually never clear the cutoff, so they stay put.
    pub fn notify_from(&self, path: &[PathSegment], cutoff: u64) {
        let children: Vec<Vec<PathSegment>> = {
            let inner = self.inner.lock().unwrap();
            let Some(node) = inner.root.get(path) else {
                return;
            };
            node.children
                .iter()
                .filter(|(seg, _)| seg.0 >= cutoff)
                .map(|(seg, child)| {
                    let mut paths = Vec::new();
                    let mut child_path = path.to_vec();
                    child_path.push(*seg);
                    paths.push(child_path.clone());
                    child.descendant_paths(&child_path, &mut paths);
                    paths
                })
                .flatten()
                .collect()
        };
        for p in children {
            self.notify_node(&p);
        }
    }

    /// Produce a [`dioxus_core::Subscribers`] list that subscribers can
    /// register against to receive shallow notifications on `path`.
    pub fn shallow_subscribers(&self, path: &[PathSegment]) -> dioxus_core::Subscribers {
        Arc::new(TreeSubscribers {
            tree: self.clone(),
            path: path.to_vec(),
            depth: Depth::Shallow,
        })
        .into()
    }

    /// Produce a [`dioxus_core::Subscribers`] list for deep notifications at
    /// `path` (subscribers also fire on descendant writes).
    pub fn deep_subscribers(&self, path: &[PathSegment]) -> dioxus_core::Subscribers {
        Arc::new(TreeSubscribers {
            tree: self.clone(),
            path: path.to_vec(),
            depth: Depth::Deep,
        })
        .into()
    }

    /// Register the current [`ReactiveContext`] (if any) as a subscriber at
    /// `path`. A shallow subscription only fires when `path` is written
    /// exactly; a deep subscription also fires on descendant writes.
    fn subscribe(&self, path: &[PathSegment], depth: Depth) {
        if ReactiveContext::current().is_some() {
            let subscribers: dioxus_core::Subscribers = Arc::new(TreeSubscribers {
                tree: self.clone(),
                path: path.to_vec(),
                depth,
            })
            .into();
            if let Some(rc) = ReactiveContext::current() {
                rc.subscribe(subscribers);
            }
        }
    }

    /// Mark every subscriber that cares about `path` as dirty. Fires:
    /// - shallow + deep subscribers on `path` itself,
    /// - deep subscribers on every strict ancestor of `path`,
    /// - all subscribers on every descendant of `path`.
    fn mark_dirty(&self, path: &[PathSegment]) {
        // Ancestors: only deep subscribers.
        for i in 0..path.len() {
            self.retain_at(&path[..i], |_, depth| !depth.is_deep());
        }

        // Exact path: all subscribers.
        self.retain_at(path, |_, _| false);

        // Descendants: all subscribers.
        let descendants = {
            let inner = self.inner.lock().unwrap();
            let mut out = Vec::new();
            if let Some(node) = inner.root.get(path) {
                node.descendant_paths(path, &mut out);
            }
            out
        };
        for p in descendants {
            self.retain_at(&p, |_, _| false);
        }
    }

    /// Walk subscribers at `path`, notify dirty, and keep the ones that
    /// return `true` from `keep`. All others are removed from the node.
    fn retain_at(
        &self,
        path: &[PathSegment],
        mut keep: impl FnMut(&ReactiveContext, Depth) -> bool,
    ) {
        // Take subscribers out under a short borrow so mark_dirty can rerun user code.
        let taken = {
            let mut inner = self.inner.lock().unwrap();
            match inner.root.get_mut(path) {
                Some(node) => std::mem::take(&mut node.subscribers),
                None => return,
            }
        };
        let mut kept = HashMap::new();
        for (rc, depth) in taken {
            if keep(&rc, depth) {
                kept.insert(rc, depth);
            } else {
                rc.mark_dirty();
            }
        }
        // Restore survivors; user code may have added new subscribers while
        // we were iterating, so extend rather than overwrite.
        let mut inner = self.inner.lock().unwrap();
        if let Some(node) = inner.root.get_mut(path) {
            for (rc, depth) in kept {
                node.subscribers
                    .entry(rc)
                    .and_modify(|d| *d |= depth)
                    .or_insert(depth);
            }
        }
    }
}

struct TreeSubscribers {
    tree: SubscriptionTree,
    path: Vec<PathSegment>,
    depth: Depth,
}

impl TreeSubscribers {
    fn prefixes(&self) -> impl Iterator<Item = &[PathSegment]> {
        (0..=self.path.len()).rev().map(move |n| &self.path[..n])
    }
}

impl SubscriberList for TreeSubscribers {
    fn add(&self, subscriber: ReactiveContext) {
        let mut inner = self.tree.inner.lock().unwrap();
        let node = inner.root.get_mut_or_default(&self.path);
        node.subscribers
            .entry(subscriber)
            .and_modify(|d| *d |= self.depth)
            .or_insert(self.depth);
    }

    fn remove(&self, subscriber: &ReactiveContext) {
        let mut inner = self.tree.inner.lock().unwrap();
        let mut empty = Vec::new();
        for prefix in self.prefixes() {
            if let Some(node) = inner.root.get_mut(prefix) {
                if let Some(depth) = node.subscribers.get(subscriber).copied() {
                    if prefix.len() == self.path.len() || depth.is_deep() {
                        node.subscribers.remove(subscriber);
                        if node.is_empty() {
                            empty.push(prefix.to_vec());
                        }
                    }
                }
            }
        }
        for prefix in empty {
            inner.root.prune(&prefix);
        }
    }

    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext)) {
        let inner = self.tree.inner.lock().unwrap();
        let mut seen: HashSet<ReactiveContext> = HashSet::new();
        for prefix in self.prefixes() {
            if let Some(node) = inner.root.get(prefix) {
                for (rc, depth) in &node.subscribers {
                    let include = prefix.len() == self.path.len() || depth.is_deep();
                    if include && seen.insert(*rc) {
                        f(rc);
                    }
                }
            }
        }
    }
}

/// Path-granular subscription wrapper for any accessor chain.
///
/// `Subscribed<A>` holds the inner accessor plus a shared subscription
/// tree. Every read subscribes the current `ReactiveContext` at the
/// accessor's path; every write notifies subscribers on that path.
pub struct Subscribed<A> {
    pub(crate) inner: A,
    pub(crate) tree: SubscriptionTree,
}

impl<A: Clone> Clone for Subscribed<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            tree: self.tree.clone(),
        }
    }
}

impl<A> Subscribed<A> {
    /// Wrap `inner` with a fresh subscription tree.
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            tree: SubscriptionTree::new(),
        }
    }

    /// Wrap `inner` with a shared subscription tree.
    pub fn with_tree(inner: A, tree: SubscriptionTree) -> Self {
        Self { inner, tree }
    }

    /// Borrow the shared subscription tree.
    pub fn tree(&self) -> &SubscriptionTree {
        &self.tree
    }

    fn collect_path(&self) -> PathBuffer
    where
        A: Pathed,
    {
        let mut buf = PathBuffer::new();
        self.inner.visit_path(&mut buf);
        buf
    }
}

impl<A> Access for Subscribed<A>
where
    A: Access + Pathed,
{
    type Target = A::Target;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, A::Target>> {
        // Subscribe path-granularly, then read *without* going through the
        // root's own reactive subscription. The path-tree is our only
        // reactivity source on a `Subscribed` optic.
        let path = self.collect_path();
        self.tree.subscribe(path.segments(), Depth::Shallow);
        self.inner.try_peek()
    }

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, A::Target>> {
        self.inner.try_peek()
    }
}

impl<A> AccessMut for Subscribed<A>
where
    A: AccessMut + Pathed,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>> {
        // Notify subscribers *before* returning the write guard so that any
        // reactive context that reruns in response to the mark_dirty sees the
        // updated value. This matches how `Signal::write` drop-guards behave
        // for non-granular subscribers.
        let path = self.collect_path();
        self.tree.mark_dirty(path.segments());
        self.inner.try_write()
    }
}

impl<A, T> ValueAccess<T> for Subscribed<A>
where
    A: ValueAccess<T> + Pathed,
{
    fn value(&self) -> T {
        let path = self.collect_path();
        self.tree.subscribe(path.segments(), Depth::Shallow);
        self.inner.value()
    }
}

impl<A> Pathed for Subscribed<A>
where
    A: Pathed,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.inner.visit_path(sink);
    }
}
