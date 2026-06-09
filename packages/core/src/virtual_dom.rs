//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::properties::RootProps;
use crate::root_wrapper::RootScopeWrapper;
use crate::{
    ComponentFunction, Element, Mutations, RenderTargetId,
    arena::ElementId,
    innerlude::{SchedulerMsg, ScopeOrder, ScopeState, VProps, WriteMutations},
    runtime::{Runtime, RuntimeGuard},
    scopes::ScopeId,
};
use crate::{Task, VComponent};
use crate::{innerlude::Work, scopes::LastRenderedNode};
use futures_util::StreamExt;
use slab::Slab;
use std::collections::{BTreeMap, BTreeSet};
use std::{any::Any, rc::Rc};
use tracing::instrument;

/// A virtual node system that progresses user events and diffs UI trees.
///
/// ## Guide
///
/// Components are defined as simple functions that take [`crate::properties::Properties`] and return an [`Element`].
///
/// ```rust
/// # use dioxus::prelude::*;
///
/// #[derive(Props, PartialEq, Clone)]
/// struct AppProps {
///     title: String
/// }
///
/// fn app(cx: AppProps) -> Element {
///     rsx!(
///         div {"hello, {cx.title}"}
///     )
/// }
/// ```
///
/// Components may be composed to make complex apps.
///
/// ```rust
/// # #![allow(unused)]
/// # use dioxus::prelude::*;
///
/// # #[derive(Props, PartialEq, Clone)]
/// # struct AppProps {
/// #     title: String
/// # }
///
/// static ROUTES: &str = "";
///
/// #[component]
/// fn app(cx: AppProps) -> Element {
///     rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{cx.title}" }
///         Footer {}
///     )
/// }
///
/// #[component]
/// fn NavBar( routes: &'static str) -> Element {
///     rsx! {
///         div { "Routes: {routes}" }
///     }
/// }
///
/// #[component]
/// fn Footer() -> Element {
///     rsx! { div { "Footer" } }
/// }
///
/// #[component]
/// fn Title( children: Element) -> Element {
///     rsx! {
///         div { id: "title", {children} }
///     }
/// }
/// ```
///
/// To start an app, register your renderer's writer via
/// [`VirtualDom::insert_render_target`] and call [`VirtualDom::rebuild`] to
/// queue the initial mutations required to draw the UI.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
///
/// let mut vdom = VirtualDom::new(app);
/// vdom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
/// vdom.rebuild();
/// let mutations = vdom
///     .take_render_target::<Mutations>(RenderTargetId::ROOT)
///     .unwrap();
/// assert!(!mutations.edits.is_empty());
/// ```
///
/// To call listeners inside the VirtualDom, call [`Runtime::handle_event`] with the appropriate event data.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// # let mut vdom = VirtualDom::new(app);
/// # let runtime = vdom.runtime();
/// let event = Event::new(std::rc::Rc::new(0) as std::rc::Rc<dyn std::any::Any>, true);
/// runtime.handle_event("onclick", event, ElementId(0));
/// ```
///
/// While no events are ready, call [`VirtualDom::wait_for_work`] to poll any futures inside the VirtualDom.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// # let mut vdom = VirtualDom::new(app);
/// tokio::runtime::Runtime::new().unwrap().block_on(async {
///     vdom.wait_for_work().await;
/// });
/// ```
///
/// Once work is ready, call [`VirtualDom::render_immediate`] to compute the differences between the previous
/// and current UI trees. This writes into the renderer's mutation queue without an intermediate copy.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// # let mut vdom = VirtualDom::new(app);
/// # vdom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
/// vdom.render_immediate();
/// ```
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # struct RealDom;
/// # impl RealDom {
/// #     fn new() -> Self { Self }
/// #     fn flush(&mut self, _: &Mutations) {}
/// #     async fn wait_for_event(&mut self) -> std::rc::Rc<dyn std::any::Any> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let mut real_dom = RealDom::new();
///
/// #[component]
/// fn app() -> Element {
///     rsx! {
///         div { "Hello World" }
///     }
/// }
///
/// let mut dom = VirtualDom::new(app);
/// dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
///
/// dom.rebuild();
/// real_dom.flush(dom.render_target_mut::<Mutations>(RenderTargetId::ROOT).unwrap());
///
/// loop {
///     tokio::select! {
///         _ = dom.wait_for_work() => {}
///         evt = real_dom.wait_for_event() => {
///             let evt = dioxus_core::Event::new(evt, true);
///             dom.runtime().handle_event("onclick", evt, ElementId(0))
///         },
///     }
///
///     dom.render_immediate();
///     real_dom.flush(dom.render_target_mut::<Mutations>(RenderTargetId::ROOT).unwrap());
/// }
/// # });
/// ```
///
/// ## Waiting for suspense
///
/// Because Dioxus supports suspense, you can use it for server-side rendering, static site generation, and other use cases
/// where waiting on portions of the UI to finish rendering is important. To wait for suspense, use the
/// [`VirtualDom::wait_for_suspense`] method:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// tokio::runtime::Runtime::new().unwrap().block_on(async {
///     let mut dom = VirtualDom::new(app);
///
///     dom.rebuild_in_place();
///     dom.wait_for_suspense().await;
/// });
///
/// // Render the virtual dom
/// ```
pub struct VirtualDom {
    pub(crate) scopes: Slab<ScopeState>,

    pub(crate) dirty_scopes: BTreeSet<ScopeOrder>,

    pub(crate) runtime: Rc<Runtime>,

    // The scopes that have been resolved since the last render
    pub(crate) resolved_scopes: Vec<ScopeId>,

    /// Renderer-supplied writers, keyed by `RenderTargetId`. Diff output for a
    /// given target is dispatched into the matching writer; targets without a
    /// registered writer have their mutations dropped (matches the historical
    /// `NoOpMutations` behaviour).
    pub(crate) targets: BTreeMap<RenderTargetId, Box<dyn crate::mutations::RenderTargetWriter>>,

    /// When `true`, mutations for unregistered targets lazily get a default
    /// `Mutations` collector instead of being dropped. Used by the
    /// `rebuild_to_targeted_vec` / `render_immediate_to_targeted_vec` helpers
    /// so portal targets created during diff don't lose their edits.
    pub(crate) auto_create_targets: bool,

    rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,
}

impl VirtualDom {
    /// Create a new VirtualDom with a component that does not have special props.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_core::*;
    /// fn Example() -> Element  {
    ///     rsx!( div { "hello world" } )
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(app: fn() -> Element) -> Self {
        Self::new_with_props(app, ())
    }

    /// Create a new VirtualDom with the given properties for the root component.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_core::*;
    /// #[derive(PartialEq, Props, Clone)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: SomeProps) -> Element  {
    ///     rsx! { div { "hello {cx.name}" } }
    /// }
    ///
    /// let dom = VirtualDom::new_with_props(Example, SomeProps { name: "world" });
    /// ```
    ///
    /// Note: the VirtualDom is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_core::*;
    /// # #[derive(PartialEq, Props, Clone)]
    /// # struct SomeProps {
    /// #     name: &'static str
    /// # }
    /// # fn Example(cx: SomeProps) -> Element  {
    /// #     rsx! { div { "hello {cx.name}" } }
    /// # }
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// dom.rebuild_in_place();
    /// ```
    pub fn new_with_props<P: Clone + 'static, M: 'static>(
        root: impl ComponentFunction<P, M>,
        root_props: P,
    ) -> Self {
        let render_fn = root.fn_ptr();
        let props = VProps::new(root, |_, _| true, root_props, "Root");
        Self::new_with_component(VComponent {
            name: "root",
            render_fn,
            props: Box::new(props),
        })
    }

    /// Create a new virtualdom and build it immediately
    pub fn prebuilt(app: fn() -> Element) -> Self {
        let mut dom = Self::new(app);
        dom.rebuild_in_place();
        dom
    }

    /// Create a new VirtualDom from a VComponent
    #[instrument(skip(root), level = "trace", name = "VirtualDom::new")]
    pub(crate) fn new_with_component(root: VComponent) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut dom = Self {
            rx,
            runtime: Runtime::new(tx),
            scopes: Default::default(),
            dirty_scopes: Default::default(),
            resolved_scopes: Default::default(),
            targets: BTreeMap::new(),
            auto_create_targets: false,
        };

        let root = VProps::new(
            RootScopeWrapper,
            |_, _| true,
            RootProps(root),
            "RootWrapper",
        );
        dom.new_scope(Box::new(root), "app");

        #[cfg(debug_assertions)]
        dom.register_subsecond_handler();

        dom
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get(id.0)
    }

    /// Get the single scope at the top of the VirtualDom tree that will always be around
    ///
    /// This scope has a ScopeId of 0 and is the root of the tree
    pub fn base_scope(&self) -> &ScopeState {
        self.get_scope(ScopeId::ROOT).unwrap()
    }

    /// Run a closure inside the dioxus runtime
    #[instrument(skip(self, f), level = "trace", name = "VirtualDom::in_runtime")]
    pub fn in_runtime<O>(&self, f: impl FnOnce() -> O) -> O {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        f()
    }

    /// Run a closure inside a specific scope
    pub fn in_scope<T>(&self, scope: ScopeId, f: impl FnOnce() -> T) -> T {
        self.runtime.in_scope(scope, f)
    }

    /// Build the virtualdom with a global context inserted into the base scope
    ///
    /// This is useful for what is essentially dependency injection when building the app
    pub fn with_root_context<T: Clone + 'static>(self, context: T) -> Self {
        self.base_scope().state().provide_context(context);
        self
    }

    /// Provide a context to the root scope
    pub fn provide_root_context<T: Clone + 'static>(&self, context: T) {
        self.base_scope().state().provide_context(context);
    }

    /// Build the virtualdom with a global context inserted into the base scope
    ///
    /// This method is useful for when you want to provide a context in your app without knowing its type
    pub fn insert_any_root_context(&mut self, context: Box<dyn Any>) {
        self.base_scope().state().provide_any_context(context);
    }

    /// Register a writer that will receive diff output for a given
    /// `RenderTargetId`. The root viewport uses `RenderTargetId::ROOT`; portal
    /// hosts insert their own ids on mount.
    ///
    /// Calling this twice with the same id drops the previous writer; use
    /// [`Self::take_render_target`] first to recover it.
    pub fn insert_render_target<W: WriteMutations + 'static>(
        &mut self,
        id: RenderTargetId,
        writer: W,
    ) {
        self.targets.insert(id, Box::new(writer));
    }

    /// Drop the writer for a given target. Mutations destined for an
    /// unregistered target are silently dropped during diff; use
    /// [`Self::take_render_target`] to recover the writer instead.
    pub fn remove_render_target(&mut self, id: RenderTargetId) {
        self.targets.remove(&id);
    }

    /// Borrow the writer for a target, downcast to its concrete type. Returns
    /// `None` if no writer is registered for `id` or the registered writer is
    /// not of type `W`.
    pub fn render_target_mut<W: WriteMutations + 'static>(
        &mut self,
        id: RenderTargetId,
    ) -> Option<&mut W> {
        self.targets.get_mut(&id)?.as_any_mut().downcast_mut::<W>()
    }

    /// Remove the writer at `id`, recovering ownership of its concrete type.
    /// Returns `None` if no writer is registered for `id` or the registered
    /// writer is not of type `W`.
    pub fn take_render_target<W: WriteMutations + 'static>(
        &mut self,
        id: RenderTargetId,
    ) -> Option<W> {
        let writer = self.targets.remove(&id)?;
        writer.into_any().downcast::<W>().ok().map(|b| *b)
    }

    /// Borrow `writer` at `ROOT` for the duration of a rebuild. Convenience
    /// for tests / single-target hosts that don't want to give up ownership.
    #[doc(hidden)]
    pub fn rebuild_into<W: WriteMutations>(&mut self, writer: &mut W) {
        self.render_pass(Some(writer), Self::rebuild_with_writer);
    }

    /// Borrow `writer` at `ROOT` for the duration of a [`Self::render_immediate`] call.
    #[doc(hidden)]
    pub fn render_immediate_into<W: WriteMutations>(&mut self, writer: &mut W) {
        self.process_events();
        self.render_pass(Some(writer), Self::render_immediate_with_writer);
        self.runtime.finish_render();
    }

    /// Run one diff pass (`run`), dispatching edits into the registered render
    /// targets — plus `root`, if given, which temporarily fronts for
    /// `RenderTargetId::ROOT`. Afterwards every touched writer is committed
    /// and each target's pending effects run, so consumers observe the whole
    /// pass atomically.
    fn render_pass(
        &mut self,
        mut root: Option<&mut dyn WriteMutations>,
        run: fn(&mut Self, &mut crate::mutations::DiffDispatch),
    ) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        let runtime = self.runtime.clone();
        let mut targets = std::mem::take(&mut self.targets);
        let mut dispatch = crate::mutations::DiffDispatch::new(
            &mut targets,
            root.as_deref_mut(),
            runtime,
            self.auto_create_targets,
        );

        run(self, &mut dispatch);
        drop(dispatch);

        let skip_root = root.map(|writer| {
            self.commit_root_writer(writer);
            RenderTargetId::ROOT
        });
        self.commit_targets(&mut targets, skip_root);
        self.targets = targets;

        self.drain_remaining_effects();
    }

    /// Mark all scopes as dirty. Each scope will be re-rendered.
    pub fn mark_all_dirty(&mut self) {
        let mut orders = vec![];

        for (_idx, scope) in self.scopes.iter() {
            orders.push(ScopeOrder::new(scope.state().height(), scope.id()));
        }

        for order in orders {
            self.queue_scope(order);
        }
    }

    /// Manually mark a scope as requiring a re-render
    ///
    /// Whenever the Runtime "works", it will re-render this scope
    pub fn mark_dirty(&mut self, id: ScopeId) {
        let Some(scope) = self.runtime.try_get_state(id) else {
            return;
        };

        tracing::event!(tracing::Level::TRACE, "Marking scope {:?} as dirty", id);
        let order = ScopeOrder::new(scope.height(), id);
        drop(scope);
        self.queue_scope(order);
    }

    /// Mark a task as dirty
    fn mark_task_dirty(&mut self, task: Task) {
        let Some(scope) = self.runtime.task_scope(task) else {
            return;
        };
        let Some(scope) = self.runtime.try_get_state(scope) else {
            return;
        };

        tracing::event!(
            tracing::Level::TRACE,
            "Marking task {:?} (spawned in {:?}) as dirty",
            task,
            scope.id,
        );

        let order = ScopeOrder::new(scope.height(), scope.id);
        drop(scope);
        self.queue_task(task, order);
    }

    /// Wait for the scheduler to have any work.
    ///
    /// This method polls the internal future queue, waiting for suspense nodes, tasks, or other work. This completes when
    /// any work is ready. If multiple scopes are marked dirty from a task or a suspense tree is finished, this method
    /// will exit.
    ///
    /// This method is cancel-safe, so you're fine to discard the future in a select block.
    ///
    /// This lets us poll async tasks and suspended trees during idle periods without blocking the main thread.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # fn app() -> Element { rsx! { div {} } }
    /// let dom = VirtualDom::new(app);
    /// ```
    #[instrument(skip(self), level = "trace", name = "VirtualDom::wait_for_work")]
    pub async fn wait_for_work(&mut self) {
        loop {
            // Process all events - Scopes are marked dirty, etc
            // Sometimes when wakers fire we get a slew of updates at once, so its important that we drain this completely
            self.process_events();

            // Now that we have collected all queued work, check whether any mounts need diffing.
            if self.has_dirty_scopes() {
                return;
            }

            // Make sure we set the runtime since we're running user code
            let _runtime = RuntimeGuard::new(self.runtime.clone());

            // There isn't any more work we can do synchronously. Wait for any new work to be ready
            self.wait_for_event().await;
        }
    }

    /// Wait for the next event to trigger and add it to the queue
    #[instrument(skip(self), level = "trace", name = "VirtualDom::wait_for_event")]
    async fn wait_for_event(&mut self) {
        match self.rx.next().await.expect("channel should never close") {
            SchedulerMsg::Immediate(id) => self.mark_dirty(id),
            SchedulerMsg::TaskNotified(id) => {
                // Instead of running the task immediately, we insert it into the runtime's task queue.
                // The task may be marked dirty at the same time as the scope that owns the task is dropped.
                self.mark_task_dirty(Task::from_id(id));
            }
            SchedulerMsg::EffectQueued => {}
            SchedulerMsg::AllDirty => self.mark_all_dirty(),
        };
    }

    /// Queue any pending events
    pub(crate) fn queue_events(&mut self) {
        // Prevent a task from deadlocking the runtime by repeatedly queueing itself
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                SchedulerMsg::TaskNotified(task) => self.mark_task_dirty(Task::from_id(task)),
                SchedulerMsg::EffectQueued => {}
                SchedulerMsg::AllDirty => self.mark_all_dirty(),
            }
        }
    }

    /// Process all events in the queue until there are no more left
    #[instrument(skip(self), level = "trace", name = "VirtualDom::process_events")]
    pub fn process_events(&mut self) {
        self.queue_events();

        // Now that we have collected all queued work, check whether any mounts need diffing.
        if self.has_dirty_scopes() {
            return;
        }

        self.poll_tasks()
    }

    /// Poll any queued tasks, then drain any effects whose owning scopes don't
    /// belong to a registered render target. Effects bound to a registered
    /// target wait until that target's next commit.
    #[instrument(skip(self), level = "trace", name = "VirtualDom::poll_tasks")]
    fn poll_tasks(&mut self) {
        // Make sure we set the runtime since we're running user code
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        while !self.has_dirty_scopes() {
            let Some(work) = self.pop_work() else {
                break;
            };

            match work {
                Work::PollTask(task) => {
                    _ = self.runtime.handle_task_wakeup(task);
                }
                Work::RerunScope(_) => {
                    return;
                }
            }

            self.queue_events();
            if self.has_dirty_scopes() {
                return;
            }
        }

        // Effects that were dirtied by task wakeups (e.g. a subscribed signal
        // written from a future) and that don't belong to a registered render
        // target need to fire now — there's no render pass coming to flush
        // them. Effects under registered targets fire when that target's next
        // commit runs.
        let mut orphan_effects = Vec::new();
        let mut remaining_effects = BTreeSet::new();
        while let Some(effect) = self.pop_effect() {
            let belongs_to_registered_target = self
                .runtime
                .try_get_state(effect.order.id)
                .map(|s| self.targets.contains_key(&s.target_id()))
                .unwrap_or(false);
            if belongs_to_registered_target {
                remaining_effects.insert(effect);
            } else {
                orphan_effects.push(effect);
            }
        }
        self.runtime
            .pending_effects
            .borrow_mut()
            .extend(remaining_effects);
        for effect in orphan_effects {
            effect.run();
        }
    }

    /// Rebuild the virtualdom without handling any of the mutations.
    ///
    /// Equivalent to [`Self::rebuild`] for a VDom with no registered render
    /// targets — diff still runs, every produced mutation is dropped.
    #[doc(hidden)]
    pub fn rebuild_in_place(&mut self) {
        self.rebuild();
    }

    /// [`VirtualDom::rebuild`] into a single root-target `Mutations` collector
    /// for tests.
    #[doc(hidden)]
    pub fn rebuild_to_vec(&mut self) -> Mutations {
        self.insert_render_target(RenderTargetId::ROOT, Mutations::default());
        self.rebuild();
        let collected = std::mem::take(
            self.render_target_mut::<Mutations>(RenderTargetId::ROOT)
                .expect("ROOT target was just registered"),
        );
        self.remove_render_target(RenderTargetId::ROOT);
        collected
    }

    /// [`VirtualDom::rebuild`] grouped into per-target mutation streams. Pairs
    /// with [`Self::render_immediate_to_targeted_vec`]. Pre-registers a fresh
    /// `Mutations` collector at every render target id the runtime knows
    /// about; targets created during diff get a lazy `Mutations` collector
    /// (the `auto_create_targets` flag).
    #[doc(hidden)]
    pub fn rebuild_to_targeted_vec(&mut self) -> BTreeMap<RenderTargetId, Mutations> {
        for id in self.runtime.known_render_target_ids() {
            self.insert_render_target(id, Mutations::default());
        }

        self.auto_create_targets = true;
        self.rebuild();
        self.auto_create_targets = false;

        self.drain_targeted_mutations()
    }

    /// Performs a *full* rebuild of the virtual dom, dispatching every edit to
    /// the renderer writers registered via [`Self::insert_render_target`].
    ///
    /// Tasks will not be polled with this method, nor will any events be
    /// processed from the event queue. Instead, the root component will be run
    /// once and then diffed.
    ///
    /// All state stored in components will be completely wiped away.
    ///
    /// Any templates previously registered will remain.
    #[doc(hidden)]
    #[instrument(skip(self), level = "trace", name = "VirtualDom::rebuild")]
    pub fn rebuild(&mut self) {
        self.render_pass(None, Self::rebuild_with_writer);
    }

    /// Render whatever the VirtualDom has ready as fast as possible without
    /// requiring an executor to progress suspended subtrees. Edits flow into
    /// the registered render targets; all accumulated writes are committed
    /// once at the end of the call.
    ///
    /// Suspense boundaries and other consumers that observe partial diff state
    /// rely on the whole pass landing atomically, so this only commits once the
    /// pass is complete.
    #[doc(hidden)]
    #[instrument(skip(self), level = "trace", name = "VirtualDom::render_immediate")]
    pub fn render_immediate(&mut self) {
        self.process_events();
        self.render_pass(None, Self::render_immediate_with_writer);
        self.runtime.finish_render();
    }

    fn rebuild_with_writer(&mut self, to: &mut crate::mutations::DiffDispatch) {
        let new_nodes = self
            .runtime
            .clone()
            .while_rendering(|| self.run_scope(ScopeId::ROOT));

        let new_nodes = LastRenderedNode::new(new_nodes);

        self.scopes[ScopeId::ROOT.0].last_rendered_node = Some(new_nodes.clone());

        let m = self.create_scope(Some(to), ScopeId::ROOT, new_nodes, None);
        to.append_children(ElementId::ROOT, m);
    }

    fn render_immediate_with_writer(&mut self, to: &mut crate::mutations::DiffDispatch) {
        // Tasks notified before this render are polled as part of it; tasks
        // first spawned *by* this render wait for the next scheduler pass.
        // Without the cutoff, a task that wakes itself on every poll would
        // extend the frame indefinitely.
        let initial_tasks: rustc_hash::FxHashSet<Task> = self
            .runtime
            .dirty_tasks
            .borrow()
            .iter()
            .flat_map(|dirty| dirty.tasks_queued.borrow().iter().copied().collect::<Vec<_>>())
            .collect();
        let mut deferred_tasks = Vec::new();

        while let Some(work) = self.pop_work() {
            match work {
                Work::PollTask(task) if initial_tasks.contains(&task) => {
                    _ = self.runtime.handle_task_wakeup(task);
                }
                Work::PollTask(task) => deferred_tasks.push(task),
                Work::RerunScope(scope) => {
                    self.runtime.clone().while_rendering(|| {
                        self.run_and_diff_scope(Some(to), scope.id);
                    });
                }
            }

            // Drain any dirty marks the work item produced (e.g. a rerun
            // child cancelling a suspended task dirties its suspense
            // boundary). They arrive over the scheduler channel and must
            // land in `dirty_scopes` for `pop_work` to see them, or the
            // render would stop before the DOM converged.
            self.queue_events();
        }

        for task in deferred_tasks {
            self.mark_task_dirty(task);
        }
    }

    fn commit_targets(
        &mut self,
        targets: &mut BTreeMap<RenderTargetId, Box<dyn crate::mutations::RenderTargetWriter>>,
        skip: Option<RenderTargetId>,
    ) {
        for (target_id, writer) in targets.iter_mut() {
            if Some(*target_id) == skip {
                continue;
            }
            writer.commit();
            for effect in self.runtime.drain_effects_for_target(*target_id) {
                effect.run();
            }
        }
    }

    fn commit_root_writer(&mut self, writer: &mut dyn WriteMutations) {
        writer.commit();
        for effect in self.runtime.drain_effects_for_target(RenderTargetId::ROOT) {
            effect.run();
        }
    }

    fn drain_remaining_effects(&mut self) {
        for effect in self.runtime.drain_remaining_effects() {
            effect.run();
        }
    }

    /// [`Self::render_immediate`] into a single root-target `Mutations`
    /// collector for tests.
    #[doc(hidden)]
    pub fn render_immediate_to_vec(&mut self) -> Mutations {
        self.insert_render_target(RenderTargetId::ROOT, Mutations::default());
        self.render_immediate();
        let collected = std::mem::take(
            self.render_target_mut::<Mutations>(RenderTargetId::ROOT)
                .expect("ROOT target was just registered"),
        );
        self.remove_render_target(RenderTargetId::ROOT);
        collected
    }

    /// [`Self::render_immediate`] grouped into per-target mutation streams.
    ///
    /// Pre-registers `Mutations` collectors at every known target id and
    /// turns on `auto_create_targets` so portal targets created during the
    /// diff still capture their edits.
    #[doc(hidden)]
    pub fn render_immediate_to_targeted_vec(&mut self) -> BTreeMap<RenderTargetId, Mutations> {
        for id in self.runtime.known_render_target_ids() {
            self.insert_render_target(id, Mutations::default());
        }

        self.auto_create_targets = true;
        self.render_immediate();
        self.auto_create_targets = false;

        self.drain_targeted_mutations()
    }

    fn drain_targeted_mutations(&mut self) -> BTreeMap<RenderTargetId, Mutations> {
        let ids: Vec<RenderTargetId> = self.targets.keys().copied().collect();
        let mut out = BTreeMap::new();
        for id in ids {
            if let Some(target) = self.render_target_mut::<Mutations>(id) {
                let m = std::mem::take(target);
                if !m.edits.is_empty() {
                    out.insert(id, m);
                }
            }
            self.remove_render_target(id);
        }
        out
    }

    /// Render the virtual dom, waiting for all suspense to be finished
    ///
    /// The mutations will be thrown out, so it's best to use this method for things like SSR that have async content
    ///
    /// We don't call "flush_sync" here since there's no sync work to be done. Futures will be progressed like usual,
    /// however any futures waiting on flush_sync will remain pending
    #[instrument(skip(self), level = "trace", name = "VirtualDom::wait_for_suspense")]
    pub async fn wait_for_suspense(&mut self) {
        loop {
            self.queue_events();

            if !self.suspended_tasks_remaining() && !self.has_dirty_scopes() {
                break;
            }

            self.wait_for_suspense_work().await;

            self.render_suspense_immediate().await;
        }
    }

    /// Check if there are any suspended tasks remaining
    pub fn suspended_tasks_remaining(&self) -> bool {
        self.runtime.suspended_tasks.get() > 0
    }

    /// Wait for the scheduler to have any work that should be run during suspense.
    #[doc(hidden)]
    pub async fn wait_for_suspense_work(&mut self) {
        // Wait for a work to be ready (IE new suspense leaves to pop up)
        loop {
            // Process all events - Scopes are marked dirty, etc
            // Sometimes when wakers fire we get a slew of updates at once, so its important that we drain this completely
            self.queue_events();

            // Now that we have collected all queued work, check whether any mounts need diffing.
            if self.has_dirty_scopes() {
                break;
            }

            {
                // Make sure we set the runtime since we're running user code
                let _runtime = RuntimeGuard::new(self.runtime.clone());
                // Next, run any queued tasks
                // We choose not to poll the deadline since we complete pretty quickly anyways
                let mut tasks_polled = 0;
                while let Some(task) = self.pop_task() {
                    if self.runtime.task_runs_during_suspense(task) {
                        let _ = self.runtime.handle_task_wakeup(task);
                        // Running that task may mark a higher mount as dirty. If it does, return early.
                        self.queue_events();
                        if self.has_dirty_scopes() {
                            return;
                        }
                    }
                    tasks_polled += 1;
                    // Once we have polled a few tasks, we manually yield to the scheduler to give it a chance to run other pending work
                    if tasks_polled > 32 {
                        yield_now().await;
                        tasks_polled = 0;
                    }
                }
            }

            self.wait_for_event().await;
        }
    }

    /// Render any suspense-ready dirty scopes without writing renderer
    /// mutations, returning the suspense boundaries that resolved.
    ///
    /// Used by SSR and tests to drive suspended subtrees to completion. Yields
    /// to the async scheduler periodically so it doesn't starve other work.
    pub async fn render_suspense_immediate(&mut self) -> Vec<ScopeId> {
        // Queue any new events before we start working
        self.queue_events();

        // Render whatever work needs to be rendered, unlocking new futures and suspense leaves
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        let mut work_done = 0;
        while let Some(work) = self.pop_work() {
            match work {
                Work::PollTask(task) => {
                    // During suspense, we only want to run tasks that are suspended
                    if self.runtime.task_runs_during_suspense(task) {
                        let _ = self.runtime.handle_task_wakeup(task);
                    }
                }
                Work::RerunScope(scope) => {
                    let scope_id: ScopeId = scope.id;
                    let run_scope = self
                        .runtime
                        .try_get_state(scope_id)
                        .filter(|scope| scope.should_run_during_suspense())
                        .is_some();
                    if run_scope {
                        // Run the scope and diff it without writing mutations.
                        // Pass `None::<&mut DiffDispatch>` so this branch shares
                        // its monomorphization with the streaming render path.
                        // Using `NoOpMutations` here would generate a separate
                        // mono whose "writes enabled" branches are unreachable
                        // by construction (the `Option` is always `None`),
                        // tanking per-monomorphization region coverage for the
                        // diff functions.
                        self.runtime.clone().while_rendering(|| {
                            self.run_and_diff_scope(
                                None::<&mut crate::mutations::DiffDispatch>,
                                scope_id,
                            );
                        });

                        tracing::trace!("Ran scope {:?} during suspense", scope_id);
                    } else {
                        tracing::warn!(
                            "Scope {:?} was marked as dirty, but will not rerun during suspense. Only nodes that are under a suspense boundary rerun during suspense",
                            scope_id
                        );
                    }
                }
            }

            // Queue any new events
            self.queue_events();
            work_done += 1;

            // Once we have polled a few work units, manually yield to the
            // scheduler to give it a chance to run other pending work.
            if work_done > 32 {
                yield_now().await;
                work_done = 0;
            }
        }

        self.resolved_scopes
            .sort_by_key(|&id| self.runtime.get_state(id).height);
        std::mem::take(&mut self.resolved_scopes)
    }

    /// Get the current runtime
    pub fn runtime(&self) -> Rc<Runtime> {
        self.runtime.clone()
    }

    /// Handle an event with the Virtual Dom. This method is deprecated in favor of [VirtualDom::runtime().handle_event] and will be removed in a future release.
    #[deprecated = "Use [VirtualDom::runtime().handle_event] instead"]
    pub fn handle_event(&self, name: &str, event: Rc<dyn Any>, element: ElementId, bubbling: bool) {
        let event = crate::Event::new(event, bubbling);
        self.runtime().handle_event(name, event, element);
    }

    #[cfg(debug_assertions)]
    fn register_subsecond_handler(&self) {
        let sender = self.runtime().sender.clone();
        subsecond::register_handler(std::sync::Arc::new(move || {
            _ = sender.unbounded_send(SchedulerMsg::AllDirty);
        }));
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // Drop all scopes in order of height
        let mut scopes = self.scopes.drain().collect::<Vec<_>>();
        scopes.sort_by_key(|scope| scope.state().height);
        for scope in scopes.into_iter().rev() {
            drop(scope);
        }

        // Drop the mounts, tasks, and effects, releasing any `Rc<Runtime>` references
        self.runtime.pending_effects.borrow_mut().clear();
        self.runtime.tasks.borrow_mut().clear();
        self.runtime.mounts.borrow_mut().clear();
    }
}

/// Yield control back to the async scheduler. This is used to give the scheduler a chance to run other pending work. Or cancel the task if the client has disconnected.
#[cfg(not(target_arch = "wasm32"))]
async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn::<(), _>(move |cx| {
        if !yielded {
            cx.waker().wake_by_ref();
            yielded = true;
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    })
    .await;
}

#[cfg(target_arch = "wasm32")]
async fn yield_now() {
    gloo_timers::future::TimeoutFuture::new(0).await;
}
