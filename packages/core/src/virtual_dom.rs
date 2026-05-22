//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::properties::RootProps;
use crate::root_wrapper::RootScopeWrapper;
use crate::{
    ComponentFunction, Element, Mutations, RenderTargetId, TargetedMutations,
    arena::ElementId,
    innerlude::{
        ComponentPropsDiff, DirtyFiberQueue, FiberStep, NoOpMutations, RenderCheckpoint,
        RenderCommit, RenderSchedulerDecision, RenderStats, SchedulerFairness, SchedulerMsg,
        ScopeOrder, ScopeState, SuspenseRenderStats, UpdatePriority, VProps, WriteMutations,
    },
    runtime::{Runtime, RuntimeGuard},
    scopes::ScopeId,
};
use crate::{Task, VComponent};
use crate::{innerlude::Work, scopes::LastRenderedNode};
use futures_util::StreamExt;
use slab::Slab;
use std::collections::BTreeMap;
use std::future::Future;
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
/// To start an app, create a [`VirtualDom`] and call [`VirtualDom::rebuild`] with your renderer's mutation writer
/// to queue the initial mutations required to draw the UI.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
///
/// let mut vdom = VirtualDom::new(app);
/// let mut mutations = Mutations::default();
/// vdom.rebuild(&mut mutations);
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
/// Once work is ready, call [`VirtualDom::render_concurrent`] to compute the differences between the previous
/// and current UI trees. This writes into the renderer's mutation queue without an intermediate copy.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// # let mut vdom = VirtualDom::new(app);
/// # let mut mutations = Mutations::default();
/// tokio::runtime::Runtime::new().unwrap().block_on(async {
///     vdom.render_concurrent(&mut mutations).await;
/// });
/// ```
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # struct RealDom {
/// #     mutations: Mutations,
/// # }
/// # struct Event {}
/// # impl RealDom {
/// #     fn new() -> Self {
/// #         Self { mutations: Mutations::default() }
/// #     }
/// #     fn apply(&mut self) -> &mut Mutations {
/// #         &mut self.mutations
/// #     }
/// #     fn commit(&mut self) {
/// #     }
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
///
/// dom.rebuild(real_dom.apply());
/// real_dom.commit();
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
///     dom.render_concurrent(real_dom.apply()).await;
///     real_dom.commit();
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

    pub(crate) dirty_fibers: DirtyFiberQueue,

    pub(crate) component_props_work: std::collections::VecDeque<ComponentPropsDiff>,

    pub(crate) runtime: Rc<Runtime>,

    // The scopes that have been resolved since the last render
    pub(crate) resolved_scopes: Vec<ScopeId>,

    pub(crate) render_priority: UpdatePriority,

    pub(crate) render_deferred_priority: Option<UpdatePriority>,

    pub(crate) scheduler_fairness: SchedulerFairness,

    pub(crate) commit_generation: u64,

    rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,
}

impl VirtualDom {
    /// Validate internal fiber bookkeeping against each scope's committed root node.
    #[doc(hidden)]
    pub fn check_fiber_invariants(&self) -> std::result::Result<(), String> {
        let fibers = self.runtime.fibers.borrow();
        for (_, scope) in &self.scopes {
            let Some(root) = scope.try_root_node() else {
                continue;
            };
            let mount = root.mount.get();
            let Some(mount_idx) = mount.as_usize() else {
                continue;
            };
            let Some(fiber) = fibers.get(mount_idx) else {
                return Err(format!(
                    "scope {:?} root uses missing fiber {:?}",
                    scope.id(),
                    mount
                ));
            };
            if fiber.node != *root {
                return Err(format!(
                    "scope {:?} root fiber {:?} has stale committed vnode",
                    scope.id(),
                    mount
                ));
            }
        }
        Ok(())
    }

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
            dirty_fibers: Default::default(),
            component_props_work: Default::default(),
            resolved_scopes: Default::default(),
            render_priority: UpdatePriority::Default,
            render_deferred_priority: None,
            scheduler_fairness: Default::default(),
            commit_generation: 0,
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
        self.mark_dirty_with_priority(id, UpdatePriority::Default);
    }

    /// Manually mark a scope as requiring a re-render at a specific priority.
    pub fn mark_dirty_with_priority(&mut self, id: ScopeId, priority: UpdatePriority) {
        let Some(scope) = self.runtime.try_get_state(id) else {
            return;
        };

        tracing::event!(tracing::Level::TRACE, "Marking scope {:?} as dirty", id);
        let order = ScopeOrder::with_priority(scope.height(), id, priority);
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

            // Now that we have collected all queued work, check whether any fibers need diffing.
            if self.has_dirty_fibers() {
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
            SchedulerMsg::Immediate(id, priority) => self.mark_dirty_with_priority(id, priority),
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
                SchedulerMsg::Immediate(id, priority) => {
                    self.mark_dirty_with_priority(id, priority)
                }
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

        // Now that we have collected all queued work, check whether any fibers need diffing.
        if self.has_dirty_fibers() {
            return;
        }

        self.poll_tasks()
    }

    /// Poll any queued tasks
    #[instrument(skip(self), level = "trace", name = "VirtualDom::poll_tasks")]
    fn poll_tasks(&mut self) {
        // Make sure we set the runtime since we're running user code
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        while !self.has_dirty_fibers() {
            let Some(work) = self.pop_work() else {
                break;
            };

            match work {
                Work::PollTask(task) => {
                    _ = self.runtime.handle_task_wakeup(task);
                }
                Work::RunEffect(effect) => {
                    effect.run();
                }
                Work::DiffFiber(_) | Work::DiffComponentProps(_) => {
                    return;
                }
            }

            self.queue_events();
            if self.has_dirty_fibers() {
                return;
            }
        }
    }

    /// Rebuild the virtualdom without handling any of the mutations
    ///
    /// This is useful for testing purposes and in cases where you render the output of the virtualdom without
    /// handling any of its mutations.
    #[doc(hidden)]
    pub fn rebuild_in_place(&mut self) {
        self.rebuild(&mut NoOpMutations);
    }

    /// [`VirtualDom::rebuild`] to a vector of mutations for testing purposes
    #[doc(hidden)]
    pub fn rebuild_to_vec(&mut self) -> Mutations {
        let mut mutations = Mutations::default();
        self.rebuild(&mut mutations);
        mutations
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom from scratch.
    ///
    /// The mutations item expects the RealDom's stack to be the root of the application.
    ///
    /// Tasks will not be polled with this method, nor will any events be processed from the event queue. Instead, the
    /// root component will be run once and then diffed. All updates will flow out as mutations.
    ///
    /// All state stored in components will be completely wiped away.
    ///
    /// Any templates previously registered will remain.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_core::*;
    /// fn app() -> Element {
    ///     rsx! { "hello world" }
    /// }
    ///
    /// let mut dom = VirtualDom::new(app);
    /// let mut mutations = Mutations::default();
    /// dom.rebuild(&mut mutations);
    /// ```
    #[doc(hidden)]
    #[instrument(skip(self, to), level = "trace", name = "VirtualDom::rebuild")]
    pub fn rebuild(&mut self, to: &mut impl WriteMutations) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let new_nodes = self
            .runtime
            .clone()
            .while_rendering(|| self.run_scope(ScopeId::ROOT));

        let new_nodes = LastRenderedNode::new(new_nodes);

        self.scopes[ScopeId::ROOT.0].last_rendered_node = Some(new_nodes.clone());

        // Rebuilding implies we append the created elements to the root
        let m = self.create_scope(Some(to), ScopeId::ROOT, new_nodes, None);

        to.append_children(ElementId::ROOT, m);
    }

    /// Render whatever the VirtualDom has ready as fast as possible without requiring an executor to progress
    /// suspended subtrees.
    #[doc(hidden)]
    #[instrument(skip(self, to), level = "trace", name = "VirtualDom::render_immediate")]
    pub fn render_immediate(&mut self, to: &mut impl WriteMutations) {
        // Process any events that might be pending in the queue
        // Signals marked with .write() need a chance to be handled by the effect driver
        // This also processes futures which might progress into immediately rerunning a scope
        self.process_events();

        // Next, diff any dirty fibers.
        // We choose not to poll the deadline since we complete pretty quickly anyways
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        while let Some(work) = self.pop_work() {
            match work {
                Work::PollTask(task) => {
                    _ = self.runtime.handle_task_wakeup(task);
                    // Make sure we process any new events
                    self.queue_events();
                }
                Work::DiffFiber(fiber) => {
                    // If the fiber is dirty, run the scope and get the mutations.
                    let priority = fiber.order.priority;
                    self.runtime.clone().while_rendering(|| {
                        fiber.diff_into(self, Some(to), priority);
                    });
                }
                Work::DiffComponentProps(diff) => {
                    self.runtime.clone().while_rendering(|| {
                        self.diff_component_props_work(diff);
                    });
                }
                Work::RunEffect(effect) => {
                    effect.run();
                }
            }
        }

        self.runtime.finish_render();
    }

    /// [`Self::render_immediate`] to a vector of mutations for testing purposes
    #[doc(hidden)]
    pub fn render_immediate_to_vec(&mut self) -> Mutations {
        let mut mutations = Mutations::default();
        self.render_immediate(&mut mutations);
        mutations
    }

    /// [`Self::render_immediate`] grouped into isolated mutation streams per render target.
    #[doc(hidden)]
    pub fn render_immediate_to_targeted_vec(&mut self) -> BTreeMap<RenderTargetId, Mutations> {
        let mut mutations = TargetedMutations::new(self.runtime.clone());
        self.render_immediate(&mut mutations);
        mutations.into_edits()
    }

    /// Render pending work into a renderer mutation queue.
    ///
    /// Commits and yields after every fiber checkpoint, leaving rescheduling
    /// to the async executor. Callers that want renderer-controlled cadence
    /// (frame timing, custom batching) should use
    /// [`Self::render_concurrent_with_scheduler`].
    #[instrument(
        skip(self, to),
        level = "trace",
        name = "VirtualDom::render_concurrent"
    )]
    pub async fn render_concurrent(&mut self, to: &mut impl WriteMutations) -> RenderStats {
        self.render_concurrent_with_scheduler(
            to,
            |_, _| RenderSchedulerDecision::CommitAndYield,
            |_, _| {},
            |_| yield_now(),
        )
        .await
    }

    /// Render pending work with renderer-controlled cooperative scheduling.
    ///
    /// The renderer receives a checkpoint after every completed scheduler work
    /// unit. This lets browser renderers tie commits and yields to frame timing
    /// instead of a fixed amount of virtual DOM work.
    #[doc(hidden)]
    #[instrument(
        skip(self, to, scheduler, commit, wait_for_scheduler),
        level = "trace",
        name = "VirtualDom::render_concurrent_with_scheduler"
    )]
    pub async fn render_concurrent_with_scheduler<M, S, C, W, F>(
        &mut self,
        to: &mut M,
        mut scheduler: S,
        mut commit: C,
        mut wait_for_scheduler: W,
    ) -> RenderStats
    where
        M: WriteMutations,
        S: FnMut(RenderCheckpoint, &M) -> RenderSchedulerDecision,
        C: FnMut(&mut M, RenderCommit),
        W: FnMut(Option<UpdatePriority>) -> F,
        F: Future<Output = ()>,
    {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let mut driver = self.fiber_driver();
        loop {
            match driver.next_fiber() {
                FiberStep::Ran(checkpoint) => {
                    let render_checkpoint = RenderCheckpoint {
                        priority: checkpoint.work.priority,
                        scope: checkpoint.work.scope,
                        work_units_since_yield: checkpoint.work_count,
                        pending_mutations: checkpoint.pending_mutations,
                        has_higher_priority_work: checkpoint.has_higher_priority_work,
                    };

                    match scheduler(render_checkpoint, to) {
                        RenderSchedulerDecision::Continue => {}
                        RenderSchedulerDecision::Commit => {
                            if let Some(fiber_commit) = driver.commit(to) {
                                commit(to, fiber_commit.into());
                            }
                        }
                        RenderSchedulerDecision::Yield => {
                            driver.yield_now();
                            wait_for_scheduler(None).await;
                        }
                        RenderSchedulerDecision::CommitAndYield => {
                            if let Some(fiber_commit) = driver.commit(to) {
                                let priority = fiber_commit.priority;
                                commit(to, fiber_commit.into());
                                driver.yield_now();
                                wait_for_scheduler(Some(priority)).await;
                            } else {
                                driver.yield_now();
                                wait_for_scheduler(None).await;
                            }
                        }
                    }
                }
                FiberStep::MustCommit => {
                    if let Some(fiber_commit) = driver.commit(to) {
                        commit(to, fiber_commit.into());
                    }
                }
                FiberStep::Idle(stats) => return stats,
            }
        }
    }

    pub(crate) fn render_work_into(
        &mut self,
        to: &mut impl WriteMutations,
        work: Work,
    ) -> UpdatePriority {
        let priority = work.priority();
        match work {
            Work::PollTask(task) => {
                _ = self.runtime.handle_task_wakeup(task);
            }
            Work::DiffFiber(fiber) => {
                self.runtime.clone().while_rendering(|| {
                    fiber.diff_into(self, Some(to), priority);
                });
            }
            Work::DiffComponentProps(diff) => {
                self.runtime.clone().while_rendering(|| {
                    self.diff_component_props_work(diff);
                });
            }
            Work::RunEffect(effect) => {
                effect.run();
            }
        }
        priority
    }

    #[allow(dead_code)]
    fn render_work_into_direct(
        &mut self,
        to: &mut impl WriteMutations,
        work: Work,
    ) -> UpdatePriority {
        let priority = work.priority();
        match work {
            Work::PollTask(task) => {
                _ = self.runtime.handle_task_wakeup(task);
            }
            Work::DiffFiber(fiber) => {
                self.runtime.clone().while_rendering(|| {
                    fiber.diff_into(self, Some(to), priority);
                });
            }
            Work::DiffComponentProps(diff) => {
                self.runtime.clone().while_rendering(|| {
                    self.diff_component_props_work(diff);
                });
            }
            Work::RunEffect(effect) => {
                effect.run();
            }
        }
        priority
    }

    fn diff_component_props_work(&mut self, diff: ComponentPropsDiff) {
        for update in diff.updates {
            let Some(scope_state) = self.runtime.try_get_state(update.scope) else {
                continue;
            };
            let height = scope_state.height;
            drop(scope_state);

            let scope_order = ScopeOrder::new(height, update.scope);
            let scope_dirty = self.dirty_fibers.contains(&scope_order);
            let old_props = &mut *self.scopes[update.scope.0].props;
            if old_props.memoize(update.props.props()) && !scope_dirty {
                continue;
            }

            self.queue_scope(ScopeOrder::with_priority(
                height,
                update.scope,
                diff.priority,
            ));
        }
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

            if !self.suspended_tasks_remaining() && !self.has_dirty_fibers() {
                break;
            }

            self.wait_for_suspense_work().await;

            self.render_suspense_concurrent().await;
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

            // Now that we have collected all queued work, check whether any fibers need diffing.
            if self.has_dirty_fibers() {
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
                        // Running that task may mark a higher fiber as dirty. If it does, return early.
                        self.queue_events();
                        if self.has_dirty_fibers() {
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

    /// Render suspense work synchronously and return the list of suspense boundaries that resolved.
    ///
    /// Equivalent to [`Self::render_suspense_concurrent`], but returns the
    /// resolved scope list directly. Used by tests and callers that want to drive suspense without
    /// caring about render stats.
    pub async fn render_suspense_immediate(&mut self) -> Vec<ScopeId> {
        self.render_suspense_concurrent().await.resolved_scopes
    }

    /// Render any suspense-ready dirty fibers without writing renderer mutations.
    ///
    /// Yields to the async scheduler after every work unit; the executor is
    /// responsible for deciding whether other tasks should run.
    pub async fn render_suspense_concurrent(&mut self) -> SuspenseRenderStats {
        // Queue any new events before we start working
        self.queue_events();

        // Render whatever work needs to be rendered, unlocking new futures and suspense leaves
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        let mut stats = RenderStats::default();
        while let Some(work) = self.pop_work() {
            let priority = work.priority();
            match work {
                Work::PollTask(task) => {
                    // During suspense, we only want to run tasks that are suspended
                    if self.runtime.task_runs_during_suspense(task) {
                        let _ = self.runtime.handle_task_wakeup(task);
                    }
                }
                Work::DiffFiber(fiber) => {
                    let scope_id: ScopeId = fiber.scope;
                    let run_scope = self
                        .runtime
                        .try_get_state(scope_id)
                        .filter(|scope| scope.should_run_during_suspense())
                        .is_some();
                    if run_scope {
                        // If the fiber is dirty, run the scope and diff it without writing mutations.
                        let priority = fiber.order.priority;
                        self.runtime.clone().while_rendering(|| {
                            fiber.diff_into(self, None::<&mut NoOpMutations>, priority);
                        });

                        tracing::trace!("Ran scope {:?} during suspense", scope_id);
                    } else {
                        tracing::warn!(
                            "Scope {:?} was marked as dirty, but will not rerun during suspense. Only nodes that are under a suspense boundary rerun during suspense",
                            scope_id
                        );
                    }
                }
                Work::DiffComponentProps(diff) => {
                    self.runtime.clone().while_rendering(|| {
                        self.diff_component_props_work(diff);
                    });
                }
                Work::RunEffect(effect) => {
                    effect.run();
                }
            }

            // Queue any new events
            self.queue_events();
            stats.priority = stats.priority.min(priority);
            stats.work_count += 1;

            // Hand back to the async scheduler — the executor decides whether
            // anything else gets to run before we resume.
            stats.yield_count += 1;
            yield_now().await;
        }

        self.resolved_scopes
            .sort_by_key(|&id| self.runtime.get_state(id).height);
        SuspenseRenderStats {
            render: stats,
            resolved_scopes: std::mem::take(&mut self.resolved_scopes),
        }
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

        // Drop the fibers, tasks, and effects, releasing any `Rc<Runtime>` references
        self.runtime.pending_effects.borrow_mut().clear();
        self.runtime.tasks.borrow_mut().clear();
        self.runtime.fibers.borrow_mut().clear();
        self.component_props_work.clear();
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
