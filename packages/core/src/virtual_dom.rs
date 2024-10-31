//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::innerlude::Work;
use crate::properties::RootProps;
use crate::root_wrapper::RootScopeWrapper;
use crate::{
    arena::ElementId,
    innerlude::{NoOpMutations, SchedulerMsg, ScopeOrder, ScopeState, VProps, WriteMutations},
    runtime::{Runtime, RuntimeGuard},
    scopes::ScopeId,
    ComponentFunction, Element, Mutations,
};
use crate::{Task, VComponent};
use futures_util::StreamExt;
use slab::Slab;
use std::collections::BTreeSet;
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
/// To start an app, create a [`VirtualDom`] and call [`VirtualDom::rebuild`] to get the list of edits required to
/// draw the UI.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
///
/// let mut vdom = VirtualDom::new(app);
/// let edits = vdom.rebuild_to_vec();
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
/// Once work is ready, call [`VirtualDom::render_immediate`] to compute the differences between the previous and
/// current UI trees. This will write edits to a [`WriteMutations`] object you pass in that contains with edits that need to be
/// handled by the renderer.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # fn app() -> Element { rsx! { div {} } }
/// # let mut vdom = VirtualDom::new(app);
/// let mut mutations = Mutations::default();
///
/// vdom.render_immediate(&mut mutations);
/// ```
///
/// To not wait for suspense while diffing the VirtualDom, call [`VirtualDom::render_immediate`].
///
///
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::*;
/// # struct RealDom;
/// # struct Event {}
/// # impl RealDom {
/// #     fn new() -> Self {
/// #         Self {}
/// #     }
/// #     fn apply(&mut self) -> Mutations {
/// #         unimplemented!()
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
/// dom.rebuild(&mut real_dom.apply());
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
///     dom.render_immediate(&mut real_dom.apply());
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
        Self::new_with_props(
            move || {
                use warnings::Warning;
                // The root props don't come from a vcomponent so we need to manually rerun them sometimes
                crate::properties::component_called_as_function::allow(app)
            },
            (),
        )
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
        let render_fn = root.id();
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

    /// Create a new VirtualDom from something that implements [`AnyProps`]
    #[instrument(skip(root), level = "trace", name = "VirtualDom::new")]
    pub(crate) fn new_with_component(root: VComponent) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut dom = Self {
            rx,
            runtime: Runtime::new(tx),
            scopes: Default::default(),
            dirty_scopes: Default::default(),
            resolved_scopes: Default::default(),
        };

        let root = VProps::new(
            RootScopeWrapper,
            |_, _| true,
            RootProps(root),
            "RootWrapper",
        );
        dom.new_scope(Box::new(root), "app");

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

    /// Manually mark a scope as requiring a re-render
    ///
    /// Whenever the Runtime "works", it will re-render this scope
    pub fn mark_dirty(&mut self, id: ScopeId) {
        let Some(scope) = self.runtime.get_state(id) else {
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
        let Some(scope) = self.runtime.get_state(scope) else {
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

            // Now that we have collected all queued work, we should check if we have any dirty scopes. If there are not, then we can poll any queued futures
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
        };
    }

    /// Queue any pending events
    fn queue_events(&mut self) {
        // Prevent a task from deadlocking the runtime by repeatedly queueing itself
        while let Ok(Some(msg)) = self.rx.try_next() {
            match msg {
                SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                SchedulerMsg::TaskNotified(task) => self.mark_task_dirty(Task::from_id(task)),
                SchedulerMsg::EffectQueued => {}
            }
        }
    }

    /// Process all events in the queue until there are no more left
    #[instrument(skip(self), level = "trace", name = "VirtualDom::process_events")]
    pub fn process_events(&mut self) {
        self.queue_events();

        // Now that we have collected all queued work, we should check if we have any dirty scopes. If there are not, then we can poll any queued futures
        if self.has_dirty_scopes() {
            return;
        }

        self.poll_tasks()
    }

    /// Poll any queued tasks
    #[instrument(skip(self), level = "trace", name = "VirtualDom::poll_tasks")]
    fn poll_tasks(&mut self) {
        // Make sure we set the runtime since we're running user code
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        // Keep polling tasks until there are no more effects or tasks to run
        // Or until we have no more dirty scopes
        while !self.runtime.dirty_tasks.borrow().is_empty()
            || !self.runtime.pending_effects.borrow().is_empty()
        {
            // Next, run any queued tasks
            // We choose not to poll the deadline since we complete pretty quickly anyways
            while let Some(task) = self.pop_task() {
                let _ = self.runtime.handle_task_wakeup(task);

                // Running that task, may mark a scope higher up as dirty. If it does, return from the function early
                self.queue_events();
                if self.has_dirty_scopes() {
                    return;
                }
            }

            // At this point, we have finished running all tasks that are pending and we haven't found any scopes to rerun. This means it is safe to run our lowest priority work: effects
            while let Some(effect) = self.pop_effect() {
                effect.run(&self.runtime);
                // Check if any new scopes are queued for rerun
                self.queue_events();
                if self.has_dirty_scopes() {
                    return;
                }
            }
        }
    }

    /// Rebuild the virtualdom without handling any of the mutations
    ///
    /// This is useful for testing purposes and in cases where you render the output of the virtualdom without
    /// handling any of its mutations.
    pub fn rebuild_in_place(&mut self) {
        self.rebuild(&mut NoOpMutations);
    }

    /// [`VirtualDom::rebuild`] to a vector of mutations for testing purposes
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
    #[instrument(skip(self, to), level = "trace", name = "VirtualDom::rebuild")]
    pub fn rebuild(&mut self, to: &mut impl WriteMutations) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let new_nodes = self.run_scope(ScopeId::ROOT);

        self.scopes[ScopeId::ROOT.0].last_rendered_node = Some(new_nodes.clone());

        // Rebuilding implies we append the created elements to the root
        let m = self.create_scope(Some(to), ScopeId::ROOT, new_nodes, None);

        to.append_children(ElementId(0), m);
    }

    /// Render whatever the VirtualDom has ready as fast as possible without requiring an executor to progress
    /// suspended subtrees.
    #[instrument(skip(self, to), level = "trace", name = "VirtualDom::render_immediate")]
    pub fn render_immediate(&mut self, to: &mut impl WriteMutations) {
        // Process any events that might be pending in the queue
        // Signals marked with .write() need a chance to be handled by the effect driver
        // This also processes futures which might progress into immediately rerunning a scope
        self.process_events();

        // Next, diff any dirty scopes
        // We choose not to poll the deadline since we complete pretty quickly anyways
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        while let Some(work) = self.pop_work() {
            match work {
                Work::PollTask(task) => {
                    _ = self.runtime.handle_task_wakeup(task);
                    // Make sure we process any new events
                    self.queue_events();
                }
                Work::RerunScope(scope) => {
                    // If the scope is dirty, run the scope and get the mutations
                    self.run_and_diff_scope(Some(to), scope.id);
                }
            }
        }

        self.runtime.finish_render();
    }

    /// [`Self::render_immediate`] to a vector of mutations for testing purposes
    pub fn render_immediate_to_vec(&mut self) -> Mutations {
        let mut mutations = Mutations::default();
        self.render_immediate(&mut mutations);
        mutations
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
            if !self.suspended_tasks_remaining() {
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
    pub async fn wait_for_suspense_work(&mut self) {
        // Wait for a work to be ready (IE new suspense leaves to pop up)
        loop {
            // Process all events - Scopes are marked dirty, etc
            // Sometimes when wakers fire we get a slew of updates at once, so its important that we drain this completely
            self.queue_events();

            // Now that we have collected all queued work, we should check if we have any dirty scopes. If there are not, then we can poll any queued futures
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
                        // Running that task, may mark a scope higher up as dirty. If it does, return from the function early
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

    /// Render any dirty scopes immediately, but don't poll any futures that are client only on that scope
    /// Returns a list of suspense boundaries that were resolved
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
                        .get_state(scope.id)
                        .filter(|scope| scope.should_run_during_suspense())
                        .is_some();
                    if run_scope {
                        // If the scope is dirty, run the scope and get the mutations
                        self.run_and_diff_scope(None::<&mut NoOpMutations>, scope_id);

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
            // Once we have polled a few tasks, we manually yield to the scheduler to give it a chance to run other pending work
            if work_done > 32 {
                yield_now().await;
                work_done = 0;
            }
        }

        self.resolved_scopes
            .sort_by_key(|&id| self.runtime.get_state(id).unwrap().height);
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
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // Drop all scopes in order of height
        let mut scopes = self.scopes.drain().collect::<Vec<_>>();
        scopes.sort_by_key(|scope| scope.state().height);
        for scope in scopes.into_iter().rev() {
            drop(scope);
        }
    }
}

/// Yield control back to the async scheduler. This is used to give the scheduler a chance to run other pending work. Or cancel the task if the client has disconnected.
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
