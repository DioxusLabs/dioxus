//! The reactive runtime: a thread-local slab of nodes (stores, memos,
//! effects, resources) with path-granular invalidation.
//!
//! # Safety model
//!
//! Every reactive value lives in a `Box<RefCell<dyn Any>>`. The box gives the
//! value a stable heap address (our stand-in for `Pin`), so guards can hold a
//! lifetime-erased `Ref<'static, _>`/`RefMut<'static, _>` into it while the
//! slab `Vec` reallocates freely. The single invariant is that a box is never
//! freed while a guard borrows it: dropping a node checks `try_borrow_mut`
//! and quarantines still-borrowed boxes in a graveyard that is only drained
//! once their borrow flag clears.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

/// A structural path into a value: field/index segments from the root of a
/// store to the part of the value a lens points at.
pub(crate) type Path = Vec<u32>;

/// Two paths overlap when one is a prefix of the other: a write to a struct
/// invalidates readers of each field, and a write to a field invalidates
/// readers of the whole struct — but sibling fields never cross-talk.
pub(crate) fn paths_overlap(a: &[u32], b: &[u32]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// A generational handle to a node in the runtime slab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId {
    index: u32,
    generation: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum NodeState {
    Clean,
    /// A transitive dependency changed; whether this node must rerun is
    /// resolved by comparing dependency versions (`needs_run`).
    Check,
    /// A direct dependency changed; this node must rerun.
    Dirty,
}

pub(crate) enum UpdateResult {
    Changed,
    Unchanged,
}

struct Dep {
    node: NodeId,
    seen_version: u64,
}

struct Sub {
    node: NodeId,
    path: Path,
}

type UpdateFn = Box<dyn FnMut() -> UpdateResult>;
type Driver = Pin<Box<dyn Future<Output = ()>>>;

struct Node {
    value: Option<Box<RefCell<dyn Any>>>,
    update: RefCell<Option<UpdateFn>>,
    driver: RefCell<Option<Driver>>,
    /// Eager nodes (effects, resources) are queued and rerun during `flush`;
    /// lazy nodes (memos) recompute on read.
    eager: bool,
    /// Ownership depth (length of the owner chain). Eager nodes run
    /// shallowest-first so a parent effect that would drop its children runs
    /// before the children do.
    depth: u32,
    state: Cell<NodeState>,
    computing: Cell<bool>,
    version: Cell<u64>,
    deps: RefCell<Vec<Dep>>,
    subs: RefCell<Vec<Sub>>,
    owned: RefCell<Vec<NodeId>>,
}

struct Slot {
    generation: u32,
    node: Option<Node>,
}

struct WakeFlag(AtomicBool);

impl Wake for WakeFlag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

struct ResourceEntry {
    id: NodeId,
    flag: Arc<WakeFlag>,
}

#[derive(Default)]
pub(crate) struct Runtime {
    slots: RefCell<Vec<Slot>>,
    free: RefCell<Vec<u32>>,
    observers: RefCell<Vec<NodeId>>,
    queue: RefCell<VecDeque<NodeId>>,
    resources: RefCell<Vec<ResourceEntry>>,
    graveyard: RefCell<Vec<Box<RefCell<dyn Any>>>>,
    flushing: Cell<bool>,
}

thread_local! {
    static RUNTIME: Runtime = Runtime::default();
}

pub(crate) fn with_rt<R>(f: impl FnOnce(&Runtime) -> R) -> R {
    RUNTIME.with(f)
}

impl Runtime {
    fn try_with_node<R>(&self, id: NodeId, f: impl FnOnce(&Node) -> R) -> Option<R> {
        let slots = self.slots.borrow();
        let slot = slots.get(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.node.as_ref().map(f)
    }

    fn with_node<R>(&self, id: NodeId, f: impl FnOnce(&Node) -> R) -> R {
        self.try_with_node(id, f)
            .expect("reactive value used after its owning scope was dropped")
    }

    pub(crate) fn is_alive(&self, id: NodeId) -> bool {
        self.try_with_node(id, |_| ()).is_some()
    }

    pub(crate) fn create_node(
        &self,
        value: Option<Box<RefCell<dyn Any>>>,
        update: Option<UpdateFn>,
        eager: bool,
    ) -> NodeId {
        let depth = self.observers.borrow().len() as u32;
        let node = Node {
            value,
            update: RefCell::new(update),
            driver: RefCell::new(None),
            eager,
            depth,
            state: Cell::new(NodeState::Dirty),
            computing: Cell::new(false),
            version: Cell::new(0),
            deps: RefCell::new(Vec::new()),
            subs: RefCell::new(Vec::new()),
            owned: RefCell::new(Vec::new()),
        };
        let id = {
            let mut slots = self.slots.borrow_mut();
            if let Some(index) = self.free.borrow_mut().pop() {
                let slot = &mut slots[index as usize];
                slot.node = Some(node);
                NodeId {
                    index,
                    generation: slot.generation,
                }
            } else {
                slots.push(Slot {
                    generation: 0,
                    node: Some(node),
                });
                NodeId {
                    index: (slots.len() - 1) as u32,
                    generation: 0,
                }
            }
        };
        if let Some(&owner) = self.observers.borrow().last() {
            self.with_node(owner, |n| n.owned.borrow_mut().push(id));
        }
        id
    }

    /// Install the update closure after node creation. Needed because the
    /// closure usually captures its own `NodeId`.
    pub(crate) fn set_update(&self, id: NodeId, update: UpdateFn) {
        self.with_node(id, |n| *n.update.borrow_mut() = Some(update));
    }

    pub(crate) fn mark_clean(&self, id: NodeId) {
        self.with_node(id, |n| n.state.set(NodeState::Clean));
    }

    /// Drop a node and everything it owns, children first. Values that are
    /// still borrowed by live guards are quarantined, not freed.
    pub(crate) fn drop_node(&self, id: NodeId) {
        let node = {
            let mut slots = self.slots.borrow_mut();
            let Some(slot) = slots.get_mut(id.index as usize) else {
                return;
            };
            if slot.generation != id.generation {
                return;
            }
            let Some(node) = slot.node.take() else {
                return;
            };
            slot.generation += 1;
            node
        };
        self.free.borrow_mut().push(id.index);
        self.resources.borrow_mut().retain(|e| e.id != id);

        let owned = std::mem::take(&mut *node.owned.borrow_mut());
        for child in owned {
            self.drop_node(child);
        }
        let deps = std::mem::take(&mut *node.deps.borrow_mut());
        for dep in deps {
            let _ = self.try_with_node(dep.node, |n| {
                n.subs.borrow_mut().retain(|s| s.node != id);
            });
        }
        if let Some(value) = node.value {
            if value.try_borrow_mut().is_ok() {
                drop(value);
            } else {
                self.graveyard.borrow_mut().push(value);
            }
        }
    }

    fn drain_graveyard(&self) {
        self.graveyard
            .borrow_mut()
            .retain(|value| value.try_borrow_mut().is_err());
    }

    pub(crate) fn current_observer(&self) -> Option<NodeId> {
        self.observers.borrow().last().copied()
    }

    /// Run `f` without an observer, so reads inside it are untracked.
    pub(crate) fn untracked<R>(&self, f: impl FnOnce() -> R) -> R {
        // A sentinel is not needed: we save and restore the whole stack depth
        // by pushing nothing and swapping the stack out instead.
        let saved = std::mem::take(&mut *self.observers.borrow_mut());
        let result = f();
        *self.observers.borrow_mut() = saved;
        result
    }

    /// Record `observer reads (node, path)`. Called lazily from guard deref.
    pub(crate) fn track_read(&self, observer: NodeId, id: NodeId, path: &[u32]) {
        if observer == id || !self.is_alive(observer) {
            return;
        }
        let version = self.with_node(id, |n| n.version.get());
        self.with_node(observer, |n| {
            let mut deps = n.deps.borrow_mut();
            if !deps.iter().any(|d| d.node == id) {
                deps.push(Dep {
                    node: id,
                    seen_version: version,
                });
            }
        });
        self.with_node(id, |n| {
            let mut subs = n.subs.borrow_mut();
            if !subs.iter().any(|s| s.node == observer && s.path == path) {
                subs.push(Sub {
                    node: observer,
                    path: path.to_vec(),
                });
            }
        });
    }

    /// Borrow a node's value cell, erasing the lifetime.
    ///
    /// # Safety of the transmute
    ///
    /// The `Ref` points into the node's `Box<RefCell<dyn Any>>` heap
    /// allocation, whose address is stable for the life of the box. The box
    /// is only freed when no borrow is outstanding (see [`Self::drop_node`]),
    /// so the erased `Ref` can never dangle.
    pub(crate) fn borrow_value(&self, id: NodeId) -> Ref<'static, dyn Any> {
        self.with_node(id, |n| {
            let cell: &RefCell<dyn Any> = n.value.as_ref().expect("node has no value cell");
            let borrowed: Ref<'_, dyn Any> = cell
                .try_borrow()
                .expect("value is mutably borrowed; do not hold a WriteGuard while reading");
            unsafe { std::mem::transmute::<Ref<'_, dyn Any>, Ref<'static, dyn Any>>(borrowed) }
        })
    }

    /// Mutably borrow a node's value cell, erasing the lifetime.
    /// See [`Self::borrow_value`] for the safety argument.
    pub(crate) fn borrow_value_mut(&self, id: NodeId) -> RefMut<'static, dyn Any> {
        self.with_node(id, |n| {
            let cell: &RefCell<dyn Any> = n.value.as_ref().expect("node has no value cell");
            let borrowed: RefMut<'_, dyn Any> = cell.try_borrow_mut().expect(
                "value is already borrowed; do not hold a guard across a write to the same value",
            );
            unsafe {
                std::mem::transmute::<RefMut<'_, dyn Any>, RefMut<'static, dyn Any>>(borrowed)
            }
        })
    }

    pub(crate) fn set_driver(&self, id: NodeId, driver: Driver) {
        self.with_node(id, |n| *n.driver.borrow_mut() = Some(driver));
        let flag = Arc::new(WakeFlag(AtomicBool::new(true)));
        let mut resources = self.resources.borrow_mut();
        resources.retain(|e| e.id != id);
        resources.push(ResourceEntry { id, flag });
    }

    /// A value changed at `path`: bump the version, mark direct overlapping
    /// subscribers dirty (transitive subscribers get `Check`), then flush.
    pub(crate) fn notify_write(&self, id: NodeId, path: &[u32]) {
        let Some(targets) = self.try_with_node(id, |n| {
            n.version.set(n.version.get() + 1);
            n.subs
                .borrow()
                .iter()
                .filter(|s| paths_overlap(path, &s.path))
                .map(|s| s.node)
                .collect::<Vec<_>>()
        }) else {
            return;
        };
        for target in targets {
            self.mark(target, NodeState::Dirty);
        }
        self.flush();
    }

    fn mark(&self, id: NodeId, level: NodeState) {
        let Some((old, eager)) = self.try_with_node(id, |n| {
            let old = n.state.get();
            if old < level {
                n.state.set(level);
            }
            (old, n.eager)
        }) else {
            return;
        };
        if old >= level {
            return;
        }
        if eager {
            if old == NodeState::Clean {
                self.queue.borrow_mut().push_back(id);
            }
        } else if old == NodeState::Clean {
            let subs = self.with_node(id, |n| {
                n.subs.borrow().iter().map(|s| s.node).collect::<Vec<_>>()
            });
            for sub in subs {
                self.mark(sub, NodeState::Check);
            }
        }
    }

    /// Resolve whether a `Check`/`Dirty` node actually has to rerun,
    /// recursively bringing memo dependencies up to date first and comparing
    /// their versions against the versions recorded when the deps were read.
    fn needs_run(&self, id: NodeId) -> bool {
        let state = self.with_node(id, |n| n.state.get());
        match state {
            NodeState::Clean => false,
            NodeState::Dirty => true,
            NodeState::Check => {
                let deps = self.with_node(id, |n| {
                    n.deps
                        .borrow()
                        .iter()
                        .map(|d| Dep {
                            node: d.node,
                            seen_version: d.seen_version,
                        })
                        .collect::<Vec<_>>()
                });
                for dep in deps {
                    self.ensure(dep.node);
                    let version = self.try_with_node(dep.node, |n| n.version.get());
                    if version.is_some_and(|v| v != dep.seen_version) {
                        self.with_node(id, |n| n.state.set(NodeState::Dirty));
                        return true;
                    }
                }
                self.with_node(id, |n| n.state.set(NodeState::Clean));
                false
            }
        }
    }

    /// Bring a derived node up to date. Stores are always up to date. Must be
    /// called before creating any read guard into a derived value, so that
    /// recomputation (which needs `&mut`) never races a live guard.
    pub(crate) fn ensure(&self, id: NodeId) {
        let Some((computing, has_update)) =
            self.try_with_node(id, |n| (n.computing.get(), n.update.borrow().is_some()))
        else {
            return;
        };
        if computing {
            panic!("reactive cycle detected: a memo or effect depends on its own value");
        }
        if !has_update {
            return;
        }
        if self.needs_run(id) {
            self.recompute(id);
        }
    }

    /// Rerun a derived node: unsubscribe old deps, drop owned children, run
    /// the update closure under tracking, and propagate if the value changed.
    fn recompute(&self, id: NodeId) {
        let old_deps = self.with_node(id, |n| {
            n.computing.set(true);
            // Clean *before* running: writes made by the closure itself
            // re-mark the node and are handled on the next flush iteration.
            n.state.set(NodeState::Clean);
            std::mem::take(&mut *n.deps.borrow_mut())
        });
        for dep in old_deps {
            let _ = self.try_with_node(dep.node, |n| {
                n.subs.borrow_mut().retain(|s| s.node != id);
            });
        }
        let owned = self.with_node(id, |n| std::mem::take(&mut *n.owned.borrow_mut()));
        for child in owned {
            self.drop_node(child);
        }

        let mut update = self
            .with_node(id, |n| n.update.borrow_mut().take())
            .expect("recompute called on a node without an update closure");
        self.observers.borrow_mut().push(id);
        let result = update();
        self.observers.borrow_mut().pop();
        let _ = self.try_with_node(id, |n| {
            *n.update.borrow_mut() = Some(update);
            n.computing.set(false);
        });

        if let UpdateResult::Changed = result {
            let targets = self.with_node(id, |n| {
                n.version.set(n.version.get() + 1);
                n.subs.borrow().iter().map(|s| s.node).collect::<Vec<_>>()
            });
            for target in targets {
                self.mark(target, NodeState::Dirty);
            }
        }
    }

    /// Poll one resource's driver future.
    fn poll_resource(&self, entry_index: usize) {
        let (id, flag) = {
            let resources = self.resources.borrow();
            let entry = &resources[entry_index];
            (entry.id, entry.flag.clone())
        };
        let Some(driver) = self.try_with_node(id, |n| n.driver.borrow_mut().take()) else {
            self.resources.borrow_mut().retain(|e| e.id != id);
            return;
        };
        let Some(mut driver) = driver else {
            self.resources.borrow_mut().retain(|e| e.id != id);
            return;
        };
        flag.0.store(false, Ordering::Release);
        let waker = Waker::from(flag);
        let mut context = Context::from_waker(&waker);
        match driver.as_mut().poll(&mut context) {
            Poll::Ready(()) => {
                self.resources.borrow_mut().retain(|e| e.id != id);
            }
            Poll::Pending => {
                let _ = self.try_with_node(id, |n| *n.driver.borrow_mut() = Some(driver));
            }
        }
    }

    /// Run queued eager nodes and ready resource futures until quiescent.
    pub(crate) fn flush(&self) {
        if self.flushing.replace(true) {
            return;
        }
        let mut iterations = 0u32;
        loop {
            let ready: Vec<usize> = {
                let resources = self.resources.borrow();
                (0..resources.len())
                    .rev()
                    .filter(|&i| resources[i].flag.0.load(Ordering::Acquire))
                    .collect()
            };
            let had_ready = !ready.is_empty();
            for index in ready {
                if index < self.resources.borrow().len() {
                    self.poll_resource(index);
                }
            }
            let next = {
                let mut queue = self.queue.borrow_mut();
                // Shallowest-first: parents may drop their children, whose
                // stale queue entries then get skipped.
                let best = (0..queue.len()).min_by_key(|&i| {
                    let id = queue[i];
                    self.try_with_node(id, |n| n.depth).unwrap_or(0)
                });
                best.and_then(|i| queue.remove(i))
            };
            match next {
                Some(id) => {
                    if self.is_alive(id) && self.needs_run(id) {
                        self.recompute(id);
                    }
                }
                None => {
                    if !had_ready {
                        break;
                    }
                }
            }
            iterations += 1;
            if iterations > 100_000 {
                panic!("reactive runaway: flush did not settle after 100000 iterations");
            }
        }
        self.flushing.set(false);
        self.drain_graveyard();
    }

    fn has_pending_resources(&self) -> bool {
        !self.resources.borrow().is_empty()
    }
}

/// Flush pending effects and ready resource futures.
pub fn flush() {
    with_rt(|rt| rt.flush());
}

/// Pump the runtime until no effects are queued and no resource futures
/// remain in flight, or until `timeout` elapses. Returns `true` if the
/// runtime settled.
pub fn run_until_settled(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        with_rt(|rt| rt.flush());
        if !with_rt(|rt| rt.has_pending_resources()) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
