//! The runtime. [`Ui`] owns the root state, all memo/resource values, the
//! retained DOM arena, and every reactive closure. Closures receive a
//! [`Ctx`] — a set of split `&mut` borrows over the runtime — so all access
//! to state is checked statically by the borrow checker. There is no
//! `RefCell` and no runtime borrow counting anywhere: read guards hand out
//! plain `&T`, write guards plain `&mut T`, and the only interior
//! mutability is `Cell`-based append-only logging of which paths were read
//! and written (which can never fail at runtime).

use std::any::Any;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context as TaskContext, Poll, Wake, Waker};

use crate::dom::Dom;
use crate::lens::{Lens, Path, paths_overlap};

/// What a reactive observer read: a structural path into the root state, a
/// memo, or a resource.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Source {
    State(Path),
    Memo(usize),
    Resource(usize),
}

/// A dependency edge, remembering the version of the source at read time
/// (versions are only meaningful for memo/resource sources).
#[derive(Clone, Debug)]
pub(crate) struct Dep {
    source: Source,
    version: u64,
}

/// The currently-running reactive observer.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Observer {
    Effect(usize),
    Memo(usize),
    Resource(usize),
}

/// Cell-based append-only read/write log. Guards only need `&Log`, so they
/// can coexist with the `&T`/`&mut T` they carry. `Cell<Vec<_>>` take/push/
/// put-back can never fail, unlike `RefCell` borrows.
#[derive(Default)]
pub(crate) struct Log {
    reads: Cell<Vec<(Observer, Source)>>,
    writes: Cell<Vec<Path>>,
    observers: Cell<Vec<Observer>>,
}

impl Log {
    fn current_observer(&self) -> Option<Observer> {
        let stack = self.observers.take();
        let top = stack.last().copied();
        self.observers.set(stack);
        top
    }

    fn push_observer(&self, o: Observer) {
        let mut stack = self.observers.take();
        stack.push(o);
        self.observers.set(stack);
    }

    fn pop_observer(&self) {
        let mut stack = self.observers.take();
        stack.pop();
        self.observers.set(stack);
    }

    pub(crate) fn record_read(&self, source: Source) {
        if let Some(observer) = self.current_observer() {
            let mut reads = self.reads.take();
            reads.push((observer, source));
            self.reads.set(reads);
        }
    }

    pub(crate) fn record_write(&self, path: Path) {
        let mut writes = self.writes.take();
        writes.push(path);
        self.writes.set(writes);
    }

    /// Remove and return the reads recorded for `observer`, leaving reads
    /// belonging to other (outer) observers in place.
    fn drain_reads_for(&self, observer: Observer) -> Vec<Source> {
        let reads = self.reads.take();
        let (mine, rest): (Vec<_>, Vec<_>) = reads.into_iter().partition(|(o, _)| *o == observer);
        self.reads.set(rest);
        mine.into_iter().map(|(_, s)| s).collect()
    }
}

/// A zero-copy read guard: derefs to plain `&T`. The read is recorded (once)
/// on first `Deref`, so merely creating a guard subscribes to nothing.
pub struct ReadGuard<'a, T: ?Sized> {
    value: &'a T,
    log: &'a Log,
    source: Option<Source>,
    tracked: Cell<bool>,
}

impl<T: ?Sized> std::ops::Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        if !self.tracked.replace(true)
            && let Some(source) = &self.source
        {
            self.log.record_read(source.clone());
        }
        self.value
    }
}

impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for ReadGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + std::fmt::Display> std::fmt::Display for ReadGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

/// A zero-copy write guard: derefs to plain `&mut T`. `Deref` records a read;
/// `DerefMut` flags a write. Subscribers are only notified (on guard drop) if
/// `DerefMut` was actually used, so a write guard used read-only is free.
pub struct WriteGuard<'a, T: ?Sized> {
    value: &'a mut T,
    log: &'a Log,
    path: Path,
    read_tracked: Cell<bool>,
    wrote: bool,
}

impl<T: ?Sized> std::ops::Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        if !self.read_tracked.replace(true) {
            self.log.record_read(Source::State(self.path.clone()));
        }
        self.value
    }
}

impl<T: ?Sized> std::ops::DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.wrote = true;
        self.value
    }
}

impl<T: ?Sized> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        if self.wrote {
            self.log.record_write(std::mem::take(&mut self.path));
        }
    }
}

/// Memo/resource dirtiness, propagated qk-style but lazily:
/// `Dirty` = a direct dependency definitely changed; `Check` = something
/// upstream may have changed, verify dependency versions before recomputing.
#[derive(Clone, Copy, PartialEq, Debug)]
enum NodeState {
    Clean,
    Check,
    Dirty,
}

/// A handle to a lazily-recomputed, equality-gated derived value.
pub struct Memo<T> {
    id: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Memo<T> {}

/// A handle to an eager reactive computation.
#[derive(Clone, Copy)]
pub struct Effect {
    id: usize,
}

impl Effect {
    /// The effect's slot index in its [`Ui`].
    pub fn id(&self) -> usize {
        self.id
    }
}

/// A handle to an async resource.
pub struct ResourceHandle<T> {
    id: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ResourceHandle<T> {}

/// The observable state of a [`ResourceHandle`]. Stale data is kept visible
/// while a reload is in flight.
#[derive(Clone, PartialEq, Debug)]
pub enum ResourceState<T> {
    /// No value has ever been produced.
    Pending,
    /// The latest run completed with this value.
    Ready(T),
    /// A previous value exists, but dependencies changed and a new run is in
    /// flight.
    Reloading(T),
}

impl<T> ResourceState<T> {
    /// The current value, if any (including stale data during a reload).
    pub fn value(&self) -> Option<&T> {
        match self {
            ResourceState::Pending => None,
            ResourceState::Ready(v) | ResourceState::Reloading(v) => Some(v),
        }
    }
}

/// Something owned by an effect scope, torn down when the effect reruns.
#[derive(Clone, Copy, Debug)]
enum Owned {
    Effect(usize),
    Memo(usize),
    Resource(usize),
}

/// A memo's closure recomputes into the type-erased value cell and reports
/// whether the value changed (`PartialEq`-gated).
type MemoCompute<S> = Box<dyn FnMut(&mut Ctx<'_, S>, &mut dyn Any) -> bool>;

struct MemoSlot<S: 'static> {
    /// `Option<T>` behind the `Any`.
    value: Box<dyn Any>,
    compute: Option<MemoCompute<S>>,
    state: NodeState,
    computing: bool,
    version: u64,
    deps: Vec<Dep>,
    alive: bool,
}

type EffectRun<S> = Box<dyn FnMut(&mut Ctx<'_, S>)>;

struct EffectSlot<S: 'static> {
    run: Option<EffectRun<S>>,
    deps: Vec<Dep>,
    owned: Vec<Owned>,
    depth: u32,
    state: NodeState,
    alive: bool,
}

/// A resource's driver future. It is pinned once on creation and never moved
/// out of its box afterwards — polling takes the `Pin<Box<_>>` out of the
/// slot (moving the box, not the future) so the slot table can be reborrowed
/// while polling.
type ResourceDriver = Pin<Box<dyn Future<Output = Box<dyn Any>>>>;
type ResourceSource<S> = Box<dyn FnMut(&mut Ctx<'_, S>) -> ResourceDriver>;

struct ResourceSlot<S: 'static> {
    /// `ResourceState<T>` behind the `Any`.
    value: Box<dyn Any>,
    /// Moves the current value into `Reloading` when a new run starts.
    reload: fn(&mut dyn Any),
    source: Option<ResourceSource<S>>,
    driver: Option<ResourceDriver>,
    /// Writes a completed `Box<dyn Any>` output into the value cell.
    complete: fn(&mut dyn Any, Box<dyn Any>),
    /// Set by the waker: the driver wants to be polled again.
    woken: Arc<WakeFlag>,
    version: u64,
    deps: Vec<Dep>,
    alive: bool,
}

/// A single-task waker: `wake` just flags the resource for re-polling.
struct WakeFlag(AtomicBool);

impl Wake for WakeFlag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// The reactive runtime: owns the root state and everything derived from it.
pub struct Ui<S: 'static> {
    state: S,
    log: Log,
    memos: Vec<MemoSlot<S>>,
    effects: Vec<EffectSlot<S>>,
    resources: Vec<ResourceSlot<S>>,
    dom: Dom<S>,
    /// Effects queued to rerun, by id.
    queue: Vec<usize>,
}

impl<S: 'static> Ui<S> {
    /// Create a runtime owning `state`.
    pub fn new(state: S) -> Self {
        Ui {
            state,
            log: Log::default(),
            memos: Vec::new(),
            effects: Vec::new(),
            resources: Vec::new(),
            dom: Dom::default(),
            queue: Vec::new(),
        }
    }

    fn ctx(&mut self) -> Ctx<'_, S> {
        Ctx {
            state: &mut self.state,
            log: &self.log,
            memos: &mut self.memos,
            effects: &mut self.effects,
            resources: &mut self.resources,
            dom: &mut self.dom,
            queue: &mut self.queue,
        }
    }

    /// Run `f` with a [`Ctx`] and flush all resulting updates.
    pub fn with<R>(&mut self, f: impl FnOnce(&mut Ctx<'_, S>) -> R) -> R {
        let r = f(&mut self.ctx());
        self.flush();
        r
    }

    /// Read through a lens without tracking (there is no observer at the
    /// top level anyway).
    pub fn get<L: Lens<S>>(&self, lens: L) -> &L::Target {
        lens.get(&self.state)
    }

    /// Create an eager effect. It runs immediately and reruns whenever
    /// anything it read changes.
    pub fn effect(&mut self, run: impl FnMut(&mut Ctx<'_, S>) + 'static) -> Effect {
        let e = self.ctx().effect(run);
        self.flush();
        e
    }

    /// Create a lazily-recomputed, `PartialEq`-gated memo.
    pub fn memo<T: PartialEq + 'static>(
        &mut self,
        compute: impl FnMut(&mut Ctx<'_, S>) -> T + 'static,
    ) -> Memo<T> {
        self.ctx().memo(compute)
    }

    /// Create an async resource. `source` runs synchronously with dependency
    /// tracking and returns the future to drive; when any tracked dependency
    /// changes, the in-flight future is dropped (cancelled) and a new one is
    /// started, keeping the previous value visible as
    /// [`ResourceState::Reloading`].
    pub fn resource<T, F>(
        &mut self,
        source: impl FnMut(&mut Ctx<'_, S>) -> F + 'static,
    ) -> ResourceHandle<T>
    where
        T: 'static,
        F: Future<Output = T> + 'static,
    {
        let r = self.ctx().resource(source);
        self.flush();
        r
    }

    /// Read a memo's current value, recomputing it first if needed.
    pub fn read_memo<T: 'static>(&mut self, memo: Memo<T>) -> &T {
        self.flush();
        let mut ctx = self.ctx();
        ctx.ensure_memo(memo.id);
        self.memos[memo.id]
            .value
            .downcast_ref::<Option<T>>()
            .expect("memo type mismatch")
            .as_ref()
            .expect("memo value missing")
    }

    /// Read a resource's current state.
    pub fn read_resource<T: 'static>(&mut self, resource: ResourceHandle<T>) -> &ResourceState<T> {
        self.flush();
        self.resources[resource.id]
            .value
            .downcast_ref::<ResourceState<T>>()
            .expect("resource type mismatch")
    }

    /// Write through a lens from outside any observer. Call [`Ui::flush`]
    /// afterwards (or use [`Ui::with`], which flushes automatically).
    pub fn write<L: Lens<S>>(&mut self, lens: L) -> WriteGuard<'_, L::Target> {
        let path = lens.path();
        WriteGuard {
            value: lens.get_mut(&mut self.state),
            log: &self.log,
            path,
            read_tracked: Cell::new(false),
            wrote: false,
        }
    }

    /// Apply pending writes and rerun invalidated effects until settled.
    pub fn flush(&mut self) {
        self.ctx().flush();
    }

    /// Poll resource futures and flush, repeating until nothing is pending
    /// or progress stops. Returns `true` if all resources are settled.
    pub fn run_until_settled(&mut self) -> bool {
        self.ctx().run_until_settled()
    }

    /// Render a mounted DOM node to an HTML string.
    pub fn render_to_string(&mut self, node: crate::dom::NodeId) -> String {
        self.flush();
        self.dom.render_to_string(node)
    }

    /// Mount a DOM tree, registering effects for its dynamic parts.
    pub fn mount(&mut self, node: impl Into<crate::dom::Node<S>>) -> crate::dom::NodeId {
        let node = node.into();
        let id = self.with(|ctx| crate::dom::mount(ctx, node));
        self.flush();
        id
    }
}

/// Split `&mut` borrows over a [`Ui`], handed to reactive closures. All state
/// access flows through this, so aliasing is impossible by construction.
pub struct Ctx<'a, S: 'static> {
    state: &'a mut S,
    log: &'a Log,
    memos: &'a mut Vec<MemoSlot<S>>,
    effects: &'a mut Vec<EffectSlot<S>>,
    resources: &'a mut Vec<ResourceSlot<S>>,
    pub(crate) dom: &'a mut Dom<S>,
    queue: &'a mut Vec<usize>,
}

impl<S: 'static> Ctx<'_, S> {
    fn reborrow(&mut self) -> Ctx<'_, S> {
        Ctx {
            state: self.state,
            log: self.log,
            memos: self.memos,
            effects: self.effects,
            resources: self.resources,
            dom: self.dom,
            queue: self.queue,
        }
    }

    /// Read through a lens. Tracked: the current observer subscribes to the
    /// lens' path. Multiple read guards may be alive at once; the borrow
    /// checker prevents overlap with any write guard.
    pub fn get<L: Lens<S>>(&self, lens: L) -> ReadGuard<'_, L::Target> {
        ReadGuard {
            value: lens.get(&*self.state),
            log: self.log,
            source: Some(Source::State(lens.path())),
            tracked: Cell::new(false),
        }
    }

    /// Read through a lens without subscribing.
    pub fn peek<L: Lens<S>>(&self, lens: L) -> &L::Target {
        lens.get(&*self.state)
    }

    /// Write through a lens. The write is only published (on guard drop) if
    /// `DerefMut` was used.
    pub fn write<L: Lens<S>>(&mut self, lens: L) -> WriteGuard<'_, L::Target> {
        let path = lens.path();
        WriteGuard {
            value: lens.get_mut(self.state),
            log: self.log,
            path,
            read_tracked: Cell::new(false),
            wrote: false,
        }
    }

    /// Run `f` without dependency tracking.
    pub fn untracked<R>(&mut self, f: impl FnOnce(&mut Ctx<'_, S>) -> R) -> R {
        // Push a sentinel-free scope by recording under no observer: we
        // temporarily hide the observer stack.
        let saved = self.log.observers.take();
        let r = f(&mut self.reborrow());
        self.log.observers.set(saved);
        r
    }

    /// Create a nested effect, owned by the current observer (torn down when
    /// the owner reruns).
    pub fn effect(&mut self, run: impl FnMut(&mut Ctx<'_, S>) + 'static) -> Effect {
        let depth = match self.log.current_observer() {
            Some(Observer::Effect(id)) => self.effects[id].depth + 1,
            _ => 0,
        };
        let id = self.effects.len();
        self.effects.push(EffectSlot {
            run: Some(Box::new(run)),
            deps: Vec::new(),
            owned: Vec::new(),
            depth,
            state: NodeState::Clean,
            alive: true,
        });
        self.register_owned(Owned::Effect(id));
        self.run_effect(id);
        Effect { id }
    }

    /// Create a memo, owned by the current observer if any.
    pub fn memo<T: PartialEq + 'static>(
        &mut self,
        mut compute: impl FnMut(&mut Ctx<'_, S>) -> T + 'static,
    ) -> Memo<T> {
        let id = self.memos.len();
        self.memos.push(MemoSlot {
            value: Box::new(None::<T>),
            compute: Some(Box::new(move |ctx, cell| {
                let new = compute(ctx);
                let cell = cell
                    .downcast_mut::<Option<T>>()
                    .expect("memo type mismatch");
                let changed = cell.as_ref() != Some(&new);
                *cell = Some(new);
                changed
            })),
            state: NodeState::Dirty,
            computing: false,
            version: 0,
            deps: Vec::new(),
            alive: true,
        });
        self.register_owned(Owned::Memo(id));
        Memo {
            id,
            _marker: std::marker::PhantomData,
        }
    }

    /// Read a memo, recomputing it first if stale. Tracked.
    pub fn read_memo<T: 'static>(&mut self, memo: Memo<T>) -> &T {
        self.ensure_memo(memo.id);
        self.log.record_read(Source::Memo(memo.id));
        self.memos[memo.id]
            .value
            .downcast_ref::<Option<T>>()
            .expect("memo type mismatch")
            .as_ref()
            .expect("memo value missing")
    }

    /// Create a resource, owned by the current observer if any.
    pub fn resource<T, F>(
        &mut self,
        mut source: impl FnMut(&mut Ctx<'_, S>) -> F + 'static,
    ) -> ResourceHandle<T>
    where
        T: 'static,
        F: Future<Output = T> + 'static,
    {
        let id = self.resources.len();
        self.resources.push(ResourceSlot {
            value: Box::new(ResourceState::<T>::Pending),
            reload: |cell| {
                let cell = cell
                    .downcast_mut::<ResourceState<T>>()
                    .expect("resource type mismatch");
                if let ResourceState::Ready(v) =
                    std::mem::replace(cell, ResourceState::<T>::Pending)
                {
                    *cell = ResourceState::Reloading(v);
                }
            },
            source: Some(Box::new(move |ctx| {
                let fut = source(ctx);
                Box::pin(async move { Box::new(fut.await) as Box<dyn Any> })
            })),
            driver: None,
            complete: |cell, out| {
                let cell = cell
                    .downcast_mut::<ResourceState<T>>()
                    .expect("resource type mismatch");
                let value = *out.downcast::<T>().expect("resource output mismatch");
                *cell = ResourceState::Ready(value);
            },
            woken: Arc::new(WakeFlag(AtomicBool::new(true))),
            version: 0,
            deps: Vec::new(),
            alive: true,
        });
        self.register_owned(Owned::Resource(id));
        self.start_resource(id);
        ResourceHandle {
            id,
            _marker: std::marker::PhantomData,
        }
    }

    /// Read a resource's state. Tracked.
    pub fn read_resource<T: 'static>(&mut self, resource: ResourceHandle<T>) -> &ResourceState<T> {
        self.log.record_read(Source::Resource(resource.id));
        self.resources[resource.id]
            .value
            .downcast_ref::<ResourceState<T>>()
            .expect("resource type mismatch")
    }

    fn register_owned(&mut self, owned: Owned) {
        if let Some(Observer::Effect(owner)) = self.log.current_observer() {
            self.effects[owner].owned.push(owned);
        }
    }

    fn drop_owned(&mut self, owned: Owned) {
        match owned {
            Owned::Effect(id) => {
                let slot = &mut self.effects[id];
                slot.alive = false;
                slot.run = None;
                slot.deps.clear();
                let children = std::mem::take(&mut slot.owned);
                for child in children {
                    self.drop_owned(child);
                }
            }
            Owned::Memo(id) => {
                let slot = &mut self.memos[id];
                slot.alive = false;
                slot.compute = None;
                slot.deps.clear();
            }
            Owned::Resource(id) => {
                let slot = &mut self.resources[id];
                slot.alive = false;
                slot.source = None;
                slot.driver = None;
                slot.deps.clear();
            }
        }
    }

    fn run_effect(&mut self, id: usize) {
        if !self.effects[id].alive {
            return;
        }
        // Tear down everything the previous run created.
        let owned = std::mem::take(&mut self.effects[id].owned);
        for child in owned {
            self.drop_owned(child);
        }
        self.effects[id].deps.clear();
        let Some(mut run) = self.effects[id].run.take() else {
            return;
        };
        self.log.push_observer(Observer::Effect(id));
        run(&mut self.reborrow());
        self.log.pop_observer();
        let reads = self.log.drain_reads_for(Observer::Effect(id));
        let deps = self.collect_deps(reads);
        let slot = &mut self.effects[id];
        slot.deps = deps;
        slot.state = NodeState::Clean;
        if slot.run.is_none() {
            slot.run = Some(run);
        }
    }

    /// Rerun a queued effect, but skip it if it was only transitively marked
    /// (`Check`) and none of its memo/resource dependencies actually changed.
    fn run_effect_if_stale(&mut self, id: usize) {
        if !self.effects[id].alive || self.effects[id].state == NodeState::Clean {
            // Clean effects can appear in the queue when a run triggered by
            // one entry re-queued the same effect before it finished.
            return;
        }
        if self.effects[id].state == NodeState::Check {
            let deps = self.effects[id].deps.clone();
            let mut changed = false;
            for dep in &deps {
                match &dep.source {
                    Source::State(_) => {}
                    Source::Memo(m) => {
                        self.ensure_memo(*m);
                        if self.memos[*m].version != dep.version {
                            changed = true;
                            break;
                        }
                    }
                    Source::Resource(r) => {
                        if self.resources[*r].version != dep.version {
                            changed = true;
                            break;
                        }
                    }
                }
            }
            // ensure_memo may have upgraded this effect to Dirty.
            if !changed && self.effects[id].state == NodeState::Check {
                self.effects[id].state = NodeState::Clean;
                return;
            }
        }
        self.run_effect(id);
    }

    /// Resolve read sources into dependency edges with current versions.
    fn collect_deps(&self, reads: Vec<Source>) -> Vec<Dep> {
        reads
            .into_iter()
            .map(|source| {
                let version = match &source {
                    Source::State(_) => 0,
                    Source::Memo(id) => self.memos[*id].version,
                    Source::Resource(id) => self.resources[*id].version,
                };
                Dep { source, version }
            })
            .collect()
    }

    /// Bring a memo up to date (recomputing only if a dependency actually
    /// changed).
    pub(crate) fn ensure_memo(&mut self, id: usize) {
        if !self.memos[id].alive {
            return;
        }
        if self.memos[id].computing {
            panic!("reactive cycle detected: a memo depends on its own value");
        }
        match self.memos[id].state {
            NodeState::Clean => return,
            NodeState::Check => {
                // Verify: did any upstream memo/resource actually change?
                let deps = self.memos[id].deps.clone();
                let mut changed = false;
                for dep in &deps {
                    match &dep.source {
                        Source::State(_) => {}
                        Source::Memo(m) => {
                            self.ensure_memo(*m);
                            if self.memos[*m].version != dep.version {
                                changed = true;
                                break;
                            }
                        }
                        Source::Resource(r) => {
                            if self.resources[*r].version != dep.version {
                                changed = true;
                                break;
                            }
                        }
                    }
                }
                if !changed {
                    self.memos[id].state = NodeState::Clean;
                    return;
                }
            }
            NodeState::Dirty => {}
        }
        self.recompute_memo(id);
    }

    fn recompute_memo(&mut self, id: usize) {
        let Some(mut compute) = self.memos[id].compute.take() else {
            return;
        };
        let mut value = std::mem::replace(&mut self.memos[id].value, Box::new(()));
        self.memos[id].computing = true;
        self.memos[id].deps.clear();
        self.log.push_observer(Observer::Memo(id));
        let changed = compute(&mut self.reborrow(), value.as_mut());
        self.log.pop_observer();
        let reads = self.log.drain_reads_for(Observer::Memo(id));
        let deps = self.collect_deps(reads);
        let slot = &mut self.memos[id];
        slot.computing = false;
        slot.deps = deps;
        slot.value = value;
        slot.state = NodeState::Clean;
        if slot.compute.is_none() {
            slot.compute = Some(compute);
        }
        if changed {
            self.memos[id].version += 1;
            self.notify_source_changed(Source::Memo(id));
        }
    }

    /// A memo/resource version bumped: mark direct dependents dirty, queue
    /// dependent effects, restart dependent resources, and propagate `Check`
    /// transitively through memos.
    fn notify_source_changed(&mut self, source: Source) {
        let mut dirty_memos = Vec::new();
        for (i, m) in self.memos.iter().enumerate() {
            if m.alive && m.deps.iter().any(|d| d.source == source) {
                dirty_memos.push(i);
            }
        }
        for i in &dirty_memos {
            if self.memos[*i].state == NodeState::Clean {
                self.memos[*i].state = NodeState::Dirty;
                self.propagate_check(*i);
            } else {
                self.memos[*i].state = NodeState::Dirty;
            }
        }
        let mut effects = Vec::new();
        for (i, e) in self.effects.iter().enumerate() {
            if e.alive && e.deps.iter().any(|d| d.source == source) {
                effects.push(i);
            }
        }
        for i in effects {
            self.effects[i].state = NodeState::Dirty;
            if !self.queue.contains(&i) {
                self.queue.push(i);
            }
        }
        let mut resources = Vec::new();
        for (i, r) in self.resources.iter().enumerate() {
            if r.alive && r.deps.iter().any(|d| d.source == source) {
                resources.push(i);
            }
        }
        for i in resources {
            self.restart_resource(i);
        }
    }

    /// Mark transitive memo dependents of memo `id` as `Check`.
    fn propagate_check(&mut self, id: usize) {
        let mut stack = vec![Source::Memo(id)];
        while let Some(source) = stack.pop() {
            let mut found = Vec::new();
            for (i, m) in self.memos.iter().enumerate() {
                if m.alive
                    && m.state == NodeState::Clean
                    && m.deps.iter().any(|d| d.source == source)
                {
                    found.push(i);
                }
            }
            for i in found {
                self.memos[i].state = NodeState::Check;
                stack.push(Source::Memo(i));
            }
            // Effects depending on a possibly-changed memo are queued as
            // `Check`: they verify dependency versions before rerunning.
            let mut effects = Vec::new();
            for (i, e) in self.effects.iter().enumerate() {
                if e.alive && e.deps.iter().any(|d| d.source == source) {
                    effects.push(i);
                }
            }
            for i in effects {
                if self.effects[i].state == NodeState::Clean {
                    self.effects[i].state = NodeState::Check;
                }
                if !self.queue.contains(&i) {
                    self.queue.push(i);
                }
            }
        }
    }

    fn start_resource(&mut self, id: usize) {
        if !self.resources[id].alive {
            return;
        }
        let Some(mut source) = self.resources[id].source.take() else {
            return;
        };
        self.resources[id].deps.clear();
        self.log.push_observer(Observer::Resource(id));
        let driver = source(&mut self.reborrow());
        self.log.pop_observer();
        let reads = self.log.drain_reads_for(Observer::Resource(id));
        let deps = self.collect_deps(reads);
        let slot = &mut self.resources[id];
        slot.deps = deps;
        slot.driver = Some(driver);
        slot.woken.0.store(true, Ordering::Relaxed);
        if slot.source.is_none() {
            slot.source = Some(source);
        }
    }

    fn restart_resource(&mut self, id: usize) {
        if !self.resources[id].alive {
            return;
        }
        // Cancel the in-flight future by dropping it, keep stale data
        // visible as `Reloading`.
        self.resources[id].driver = None;
        let reload = self.resources[id].reload;
        reload(self.resources[id].value.as_mut());
        self.start_resource(id);
        self.notify_source_changed(Source::Resource(id));
    }

    /// Poll every in-flight resource future whose waker fired since the last
    /// poll. Returns `true` if any was polled.
    fn poll_resources(&mut self) -> bool {
        let mut polled = false;
        for id in 0..self.resources.len() {
            if !self.resources[id].woken.0.swap(false, Ordering::Relaxed) {
                continue;
            }
            // Move the pinned box out of the slot (this moves the box, not
            // the pinned future inside it) so the tables stay reborrowable.
            let Some(mut driver) = self.resources[id].driver.take() else {
                continue;
            };
            polled = true;
            let waker = Waker::from(self.resources[id].woken.clone());
            let mut cx = TaskContext::from_waker(&waker);
            match driver.as_mut().poll(&mut cx) {
                Poll::Ready(out) => {
                    let complete = self.resources[id].complete;
                    complete(self.resources[id].value.as_mut(), out);
                    self.resources[id].version += 1;
                    self.notify_source_changed(Source::Resource(id));
                }
                Poll::Pending => {
                    self.resources[id].driver = Some(driver);
                }
            }
        }
        polled
    }

    /// Apply pending writes and rerun queued effects until settled.
    pub(crate) fn flush(&mut self) {
        loop {
            let writes = self.log.writes.take();
            if writes.is_empty() && self.queue.is_empty() {
                break;
            }
            for path in writes {
                self.apply_write(&path);
            }
            // Run the shallowest queued effect first so parents rerun (and
            // tear down stale children) before their children run.
            if !self.queue.is_empty() {
                let mut best = 0;
                for (i, id) in self.queue.iter().enumerate() {
                    if self.effects[*id].depth < self.effects[self.queue[best]].depth {
                        best = i;
                    }
                }
                let id = self.queue.remove(best);
                self.run_effect_if_stale(id);
            }
        }
    }

    fn apply_write(&mut self, path: &[u32]) {
        // Memos reading an overlapping path become Dirty.
        let mut dirty = Vec::new();
        for (i, m) in self.memos.iter().enumerate() {
            if m.alive
                && m.deps
                    .iter()
                    .any(|d| matches!(&d.source, Source::State(p) if paths_overlap(p, path)))
            {
                dirty.push(i);
            }
        }
        for i in dirty {
            self.memos[i].state = NodeState::Dirty;
            self.propagate_check(i);
        }
        // Effects reading an overlapping path are queued.
        let mut effects = Vec::new();
        for (i, e) in self.effects.iter().enumerate() {
            if e.alive
                && e.deps
                    .iter()
                    .any(|d| matches!(&d.source, Source::State(p) if paths_overlap(p, path)))
            {
                effects.push(i);
            }
        }
        for i in effects {
            self.effects[i].state = NodeState::Dirty;
            if !self.queue.contains(&i) {
                self.queue.push(i);
            }
        }
        // Resources reading an overlapping path restart.
        let mut resources = Vec::new();
        for (i, r) in self.resources.iter().enumerate() {
            if r.alive
                && r.deps
                    .iter()
                    .any(|d| matches!(&d.source, Source::State(p) if paths_overlap(p, path)))
            {
                resources.push(i);
            }
        }
        for i in resources {
            self.restart_resource(i);
        }
    }

    /// Poll woken resources and flush repeatedly until no future asks to be
    /// polled again. Returns `true` if all resources completed; `false` if
    /// some are still waiting on an external wake-up.
    pub(crate) fn run_until_settled(&mut self) -> bool {
        loop {
            self.flush();
            let progressed = self.poll_resources();
            self.flush();
            if !progressed {
                return self.resources.iter().all(|r| r.driver.is_none());
            }
        }
    }
}
