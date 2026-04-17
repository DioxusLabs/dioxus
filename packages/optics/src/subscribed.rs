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
    sync::Arc,
};

use dioxus_core::{current_owner, ReactiveContext, SubscriberList};
use generational_box::{AnyStorage, GenerationalBox, SyncStorage, WriteLock};

use crate::combinator::{Access, AccessMut, ValueAccess};
use crate::path::{PathBuffer, PathSegment, Pathed};

/// Tree node used for path-granular subscription tracking.
#[derive(Default, Clone)]
struct Node {
    subscribers: HashMap<ReactiveContext, Depth>,
    children: HashMap<PathSegment, Node>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
            [first, rest @ ..] => self.children.get_mut(first).and_then(|n| n.get_mut(rest)),
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

    fn descendant_paths(&self, current: &PathBuffer, out: &mut Vec<PathBuffer>) {
        for (seg, child) in &self.children {
            let mut p = *current;
            p.push(*seg);
            out.push(p);
            child.descendant_paths(&p, out);
        }
    }
}

#[derive(Default, Clone)]
struct TreeInner {
    root: Node,
}

/// Shared subscription tree — `Copy` (backed by a `GenerationalBox` slot
/// in sync storage so the whole optic chain composes as a `Copy` value
/// just like the old `dioxus-stores` subscriptions did).
pub struct SubscriptionTree {
    inner: GenerationalBox<TreeInner, SyncStorage>,
}

impl Copy for SubscriptionTree {}

impl Clone for SubscriptionTree {
    fn clone(&self) -> Self {
        *self
    }
}

impl Default for SubscriptionTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionTree {
    /// Create an empty subscription tree. Allocates a slot in the current
    /// Dioxus scope's sync-storage owner, so this must be called inside a
    /// hook / component initialization (matches `Store::new` / `use_store`).
    #[track_caller]
    pub fn new() -> Self {
        let owner = current_owner::<SyncStorage>();
        Self {
            inner: owner.insert_rc(TreeInner::default()),
        }
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
        let children: Vec<PathBuffer> = {
            let Ok(inner) = self.inner.try_read() else {
                return;
            };
            let Some(node) = inner.root.get(path) else {
                return;
            };
            node.children
                .iter()
                .filter(|(seg, _)| seg.0 >= cutoff)
                .flat_map(|(seg, child)| {
                    let mut paths = Vec::new();
                    let mut child_path = PathBuffer::new();
                    for s in path {
                        child_path.push(*s);
                    }
                    child_path.push(*seg);
                    paths.push(child_path);
                    child.descendant_paths(&child_path, &mut paths);
                    paths
                })
                .collect()
        };
        for p in children {
            self.notify_node(p.segments());
        }
    }

    /// Produce a [`dioxus_core::Subscribers`] list that subscribers can
    /// register against to receive shallow notifications on `path`.
    pub fn shallow_subscribers(&self, path: &[PathSegment]) -> dioxus_core::Subscribers {
        Arc::new(TreeSubscribers {
            tree: self.clone(),
            path: path_buffer_from_slice(path),
            depth: Depth::Shallow,
        })
        .into()
    }

    /// Produce a [`dioxus_core::Subscribers`] list for deep notifications at
    /// `path` (subscribers also fire on descendant writes).
    pub fn deep_subscribers(&self, path: &[PathSegment]) -> dioxus_core::Subscribers {
        Arc::new(TreeSubscribers {
            tree: self.clone(),
            path: path_buffer_from_slice(path),
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
                path: path_buffer_from_slice(path),
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
            let Ok(inner) = self.inner.try_read() else {
                return;
            };
            let mut out = Vec::new();
            if let Some(node) = inner.root.get(path) {
                let base = path_buffer_from_slice(path);
                node.descendant_paths(&base, &mut out);
            }
            out
        };
        for p in descendants {
            self.retain_at(p.segments(), |_, _| false);
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
            let Ok(mut inner) = self.inner.try_write() else {
                return;
            };
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
        let Ok(mut inner) = self.inner.try_write() else {
            return;
        };
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
    path: PathBuffer,
    depth: Depth,
}

impl TreeSubscribers {
    fn prefixes(&self) -> impl Iterator<Item = &[PathSegment]> {
        let segs = self.path.segments();
        (0..=segs.len()).rev().map(move |n| &segs[..n])
    }
}

fn path_buffer_from_slice(path: &[PathSegment]) -> PathBuffer {
    let mut buf = PathBuffer::new();
    for seg in path {
        buf.push(*seg);
    }
    buf
}

impl SubscriberList for TreeSubscribers {
    fn add(&self, subscriber: ReactiveContext) {
        // The backing `GenerationalBox` can be dropped out from under us
        // when the owning scope (and hence the `SubscriptionTree`) has
        // already torn down — e.g. during a ReactiveContext's post-scope
        // cleanup. In that case there's nothing to update; just return.
        // Matches the old `dioxus-stores` `StoreSubscribers` behavior.
        let Ok(mut inner) = self.tree.inner.try_write() else {
            return;
        };
        let node = inner.root.get_mut_or_default(self.path.segments());
        node.subscribers
            .entry(subscriber)
            .and_modify(|d| *d |= self.depth)
            .or_insert(self.depth);
    }

    fn remove(&self, subscriber: &ReactiveContext) {
        let Ok(mut inner) = self.tree.inner.try_write() else {
            return;
        };
        let mut empty: Vec<PathBuffer> = Vec::new();
        let path_len = self.path.len();
        for prefix in self.prefixes() {
            if let Some(node) = inner.root.get_mut(prefix) {
                if let Some(depth) = node.subscribers.get(subscriber).copied() {
                    if prefix.len() == path_len || depth.is_deep() {
                        node.subscribers.remove(subscriber);
                        if node.is_empty() {
                            empty.push(path_buffer_from_slice(prefix));
                        }
                    }
                }
            }
        }
        for prefix in empty {
            inner.root.prune(prefix.segments());
        }
    }

    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext)) {
        let Ok(inner) = self.tree.inner.try_read() else {
            return;
        };
        let mut seen: HashSet<ReactiveContext> = HashSet::new();
        let path_len = self.path.len();
        for prefix in self.prefixes() {
            if let Some(node) = inner.root.get(prefix) {
                for (rc, depth) in &node.subscribers {
                    let include = prefix.len() == path_len || depth.is_deep();
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
            tree: self.tree,
        }
    }
}

impl<A: Copy> Copy for Subscribed<A> {}

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

    /// Borrow the inner accessor. Useful for readable / writable bridges
    /// that want to forward `try_read` / `try_peek` calls to the underlying
    /// carrier without going through the path-subscription machinery.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Consume the wrapper and return its constituent parts — the inner
    /// accessor and the shared subscription tree. Used by `dioxus-stores`
    /// to rebuild a `Store<T, Lens>` with `Lens = A` that keeps
    /// path-granular subscriptions wired through the same tree.
    pub fn into_parts(self) -> (A, SubscriptionTree) {
        (self.inner, self.tree)
    }

    fn collect_path(&self) -> PathBuffer
    where
        A: Pathed,
    {
        let mut buf = PathBuffer::new();
        self.inner.visit_path(&mut buf);
        buf
    }

    /// Borrow the path this subscription wraps, as a fresh [`PathBuffer`].
    ///
    /// Useful for callers who want to hand the same path to another chain
    /// (e.g. to set up a sibling `Subscribed::with_tree` sharing this tree).
    pub fn path(&self) -> PathBuffer
    where
        A: Pathed,
    {
        self.collect_path()
    }

    /// Notify shallow subscribers at this carrier's exact path — does not
    /// touch descendant subscribers. Use from collection-op write paths
    /// (`push`, `clear`, `retain`) where the length/shape of the container
    /// changed but existing child values did not.
    ///
    /// Mirrors `SelectorScope::mark_dirty_shallow` on the stores side.
    pub fn notify_node_dirty(&self)
    where
        A: Pathed,
    {
        let path = self.collect_path();
        self.tree.notify_node(path.segments());
    }

    /// Notify shallow-and-deeper subscribers for children whose segment is
    /// numerically `>= cutoff`. Use for ordered-container insertion: after
    /// inserting at index `i`, every sibling from `i..len` shifted and needs
    /// to re-run.
    ///
    /// Mirrors `SelectorScope::mark_dirty_at_and_after_index` on the stores
    /// side.
    pub fn notify_from(&self, cutoff: u64)
    where
        A: Pathed,
    {
        let path = self.collect_path();
        self.tree.notify_from(path.segments(), cutoff);
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
        // root's own reactive subscription. Deep tracking matches
        // `SelectorScope::track()` so readers fire on exact-path writes **or**
        // descendant writes (e.g. subscribing to `todos` sees writes to a
        // specific todo).
        let path = self.collect_path();
        self.tree.subscribe(path.segments(), Depth::Deep);
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

    fn try_write(&self) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>> {
        // Notify subscribers *before* returning the write guard so that any
        // reactive context that reruns in response to the mark_dirty sees the
        // updated value. This matches how `Signal::write` drop-guards behave
        // for non-granular subscribers.
        let path = self.collect_path();
        self.tree.mark_dirty(path.segments());
        // Silent write on the inner to avoid a second fire at its own (broader)
        // path — `mark_dirty` already covered everyone who cares.
        self.inner.try_write_silent()
    }

    fn try_write_silent(
        &self,
    ) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>> {
        // Chain-silent write: we don't fire our tree either. The outermost
        // `Subscribed` at the call site is expected to fire explicitly.
        self.inner.try_write_silent()
    }
}

impl<A, T> ValueAccess<T> for Subscribed<A>
where
    A: ValueAccess<T> + Pathed,
{
    fn value(&self) -> T {
        let path = self.collect_path();
        self.tree.subscribe(path.segments(), Depth::Deep);
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

/// Carriers that already carry a [`SubscriptionTree`] expose it through this
/// trait so downstream combinators (e.g. `#[derive(Store)]` field accessors)
/// can rewrap projected children in a new `Subscribed` that shares the same
/// tree. Sharing is what keeps path-granular reactivity working across
/// arbitrarily deep chains without allocating a fresh tree per projection.
///
/// Non-root carriers (Signal, Memo, CopyValue, Resource, etc.) get a
/// convenience impl in the signals / resource bridges that builds a fresh
/// tree on demand — calling `.subscription_tree()` on a raw Signal is a
/// deliberate opt-in: you want tree-backed reactivity from here on.
pub trait HasSubscriptionTree {
    /// Return (or construct) the tree that subsequent projections should
    /// subscribe against. For carriers that already hold a tree this is a
    /// cheap clone of an `Arc`; for bare-reactive roots it creates a new
    /// `SubscriptionTree`.
    fn subscription_tree(&self) -> SubscriptionTree;
}

impl<A> HasSubscriptionTree for Subscribed<A> {
    fn subscription_tree(&self) -> SubscriptionTree {
        self.tree.clone()
    }
}
