//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::{
    any_props::AnyProps,
    arena::ElementId,
    innerlude::{
        DirtyScope, ElementRef, ErrorBoundary, NoOpMutations, SchedulerMsg, ScopeState, VNodeMount,
        VProps, WriteMutations,
    },
    nodes::RenderReturn,
    nodes::{Template, TemplateId},
    runtime::{Runtime, RuntimeGuard},
    scopes::ScopeId,
    AttributeValue, ComponentFunction, Element, Event, Mutations,
};
use futures_util::StreamExt;
use rustc_hash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::{any::Any, collections::BTreeSet, rc::Rc};

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
/// # fn app() -> Element { rsx! { div {} } }
///
/// let mut vdom = VirtualDom::new(app);
/// let edits = vdom.rebuild_to_vec();
/// ```
///
/// To call listeners inside the VirtualDom, call [`VirtualDom::handle_event`] with the appropriate event data.
///
/// ```rust, ignore
/// vdom.handle_event(event);
/// ```
///
/// While no events are ready, call [`VirtualDom::wait_for_work`] to poll any futures inside the VirtualDom.
///
/// ```rust, ignore
/// vdom.wait_for_work().await;
/// ```
///
/// Once work is ready, call [`VirtualDom::render_with_deadline`] to compute the differences between the previous and
/// current UI trees. This will return a [`Mutations`] object that contains Edits, Effects, and NodeRefs that need to be
/// handled by the renderer.
///
/// ```rust, ignore
/// let mutations = vdom.work_with_deadline(tokio::time::sleep(Duration::from_millis(100)));
///
/// for edit in mutations.edits {
///     real_dom.apply(edit);
/// }
/// ```
///
/// To not wait for suspense while diffing the VirtualDom, call [`VirtualDom::render_immediate`] or pass an immediately
/// ready future to [`VirtualDom::render_with_deadline`].
///
///
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
/// ```rust, ignore
/// #[component]
/// fn app() -> Element {
///     rsx! {
///         div { "Hello World" }
///     }
/// }
///
/// let dom = VirtualDom::new(app);
///
/// real_dom.apply(dom.rebuild());
///
/// loop {
///     select! {
///         _ = dom.wait_for_work() => {}
///         evt = real_dom.wait_for_event() => dom.handle_event(evt),
///     }
///
///     real_dom.apply(dom.render_immediate());
/// }
/// ```
///
/// ## Waiting for suspense
///
/// Because Dioxus supports suspense, you can use it for server-side rendering, static site generation, and other usecases
/// where waiting on portions of the UI to finish rendering is important. To wait for suspense, use the
/// [`VirtualDom::render_with_deadline`] method:
///
/// ```rust, ignore
/// let dom = VirtualDom::new(app);
///
/// let deadline = tokio::time::sleep(Duration::from_millis(100));
/// let edits = dom.render_with_deadline(deadline).await;
/// ```
///
/// ## Use with streaming
///
/// If not all rendering is done by the deadline, it might be worthwhile to stream the rest later. To do this, we
/// suggest rendering with a deadline, and then looping between [`VirtualDom::wait_for_work`] and render_immediate until
/// no suspended work is left.
///
/// ```rust, ignore
/// let dom = VirtualDom::new(app);
///
/// let deadline = tokio::time::sleep(Duration::from_millis(20));
/// let edits = dom.render_with_deadline(deadline).await;
///
/// real_dom.apply(edits);
///
/// while dom.has_suspended_work() {
///    dom.wait_for_work().await;
///    real_dom.apply(dom.render_immediate());
/// }
/// ```
pub struct VirtualDom {
    pub(crate) scopes: Slab<ScopeState>,

    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,

    // Maps a template path to a map of byte indexes to templates
    pub(crate) templates: FxHashMap<TemplateId, FxHashMap<usize, Template>>,

    // Templates changes that are queued for the next render
    pub(crate) queued_templates: Vec<Template>,

    // The element ids that are used in the renderer
    pub(crate) elements: Slab<Option<ElementRef>>,

    // Once nodes are mounted, the information about where they are mounted is stored here
    pub(crate) mounts: Slab<VNodeMount>,

    pub(crate) runtime: Rc<Runtime>,

    // Currently suspended scopes
    pub(crate) suspended_scopes: FxHashSet<ScopeId>,

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
    /// ```rust, ignore
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
    /// ```rust, ignore
    /// #[derive(PartialEq, Props)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: SomeProps) -> Element  {
    ///     rsx!{ div { "hello {cx.name}" } }
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// let mutations = dom.rebuild();
    /// ```
    pub fn new_with_props<P: Clone + 'static, M: 'static>(
        root: impl ComponentFunction<P, M>,
        root_props: P,
    ) -> Self {
        Self::new_with_component(VProps::new(root, |_, _| true, root_props, "root"))
    }

    /// Create a new virtualdom and build it immediately
    pub fn prebuilt(app: fn() -> Element) -> Self {
        let mut dom = Self::new(app);
        dom.rebuild_in_place();
        dom
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
    /// ```rust, ignore
    /// #[derive(PartialEq, Props)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: SomeProps) -> Element  {
    ///     rsx!{ div{ "hello {cx.name}" } }
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new_from_root(VComponent::new(Example, SomeProps { name: "jane" }, "Example"));
    /// let mutations = dom.rebuild();
    /// ```
    pub(crate) fn new_with_component(root: impl AnyProps + 'static) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut dom = Self {
            rx,
            runtime: Runtime::new(tx),
            scopes: Default::default(),
            dirty_scopes: Default::default(),
            templates: Default::default(),
            queued_templates: Default::default(),
            elements: Default::default(),
            mounts: Default::default(),
            suspended_scopes: Default::default(),
        };

        let root = dom.new_scope(Box::new(root), "app");

        // Unlike react, we provide a default error boundary that just renders the error as a string
        root.state()
            .provide_context(Rc::new(ErrorBoundary::new_in_scope(ScopeId::ROOT)));

        // the root element is always given element ID 0 since it's the container for the entire tree
        dom.elements.insert(None);

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

        tracing::trace!("Marking scope {:?} ({}) as dirty", id, scope.name);
        self.dirty_scopes.insert(DirtyScope {
            height: scope.height(),
            id,
        });
    }

    /// Call a listener inside the VirtualDom with data from outside the VirtualDom. **The ElementId passed in must be the id of an element with a listener, not a static node or a text node.**
    ///
    /// This method will identify the appropriate element. The data must match up with the listener declared. Note that
    /// this method does not give any indication as to the success of the listener call. If the listener is not found,
    /// nothing will happen.
    ///
    /// It is up to the listeners themselves to mark nodes as dirty.
    ///
    /// If you have multiple events, you can call this method multiple times before calling "render_with_deadline"
    pub fn handle_event(
        &mut self,
        name: &str,
        data: Rc<dyn Any>,
        element: ElementId,
        bubbles: bool,
    ) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        if let Some(Some(parent_path)) = self.elements.get(element.0).copied() {
            if bubbles {
                self.handle_bubbling_event(Some(parent_path), name, Event::new(data, bubbles));
            } else {
                self.handle_non_bubbling_event(parent_path, name, Event::new(data, bubbles));
            }
        }
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
    /// ```rust, ignore
    /// let dom = VirtualDom::new(app);
    /// ```
    pub async fn wait_for_work(&mut self) {
        // And then poll the futures
        self.poll_tasks().await;
    }

    ///
    async fn poll_tasks(&mut self) {
        // Release the flush lock
        // This will cause all the flush wakers to immediately spring to life, which we will off with process_events
        self.runtime.release_flush_lock();

        loop {
            // Process all events - Scopes are marked dirty, etc
            // Sometimes when wakers fire we get a slew of updates at once, so its important that we drain this completely
            self.process_events();

            // Now that we have collected all queued work, we should check if we have any dirty scopes. If there are not, then we can poll any queued futures
            if !self.dirty_scopes.is_empty() {
                return;
            }

            // Make sure we set the runtime since we're running user code
            let _runtime = RuntimeGuard::new(self.runtime.clone());

            // Hold a lock to the flush sync to prevent tasks from running in the event we get an immediate
            // When we're doing awaiting the rx, the lock will be dropped and tasks waiting on the lock will get waked
            // We have to own the lock since poll_tasks is cancel safe - the future that this is running in might get dropped
            // and if we held the lock in the scope, the lock would also get dropped prematurely
            self.runtime.release_flush_lock();
            self.runtime.acquire_flush_lock();

            match self.rx.next().await.expect("channel should never close") {
                SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                SchedulerMsg::TaskNotified(id) => _ = self.runtime.handle_task_wakeup(id),
            };
        }
    }

    /// Process all events in the queue until there are no more left
    pub fn process_events(&mut self) {
        let _runtime = RuntimeGuard::new(self.runtime.clone());

        // Prevent a task from deadlocking the runtime by repeatedly queueing itself
        while let Ok(Some(msg)) = self.rx.try_next() {
            match msg {
                SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                SchedulerMsg::TaskNotified(task) => _ = self.runtime.handle_task_wakeup(task),
            }
        }
    }

    /// Replace a template at runtime. This will re-render all components that use this template.
    /// This is the primitive that enables hot-reloading.
    ///
    /// The caller must ensure that the template references the same dynamic attributes and nodes as the original template.
    ///
    /// This will only replace the the parent template, not any nested templates.
    pub fn replace_template(&mut self, template: Template) {
        self.register_template_first_byte_index(template);
        // iterating a slab is very inefficient, but this is a rare operation that will only happen during development so it's fine
        for (_, scope) in self.scopes.iter() {
            if let Some(RenderReturn::Ready(sync)) = scope.try_root_node() {
                if sync.template.get().name.rsplit_once(':').unwrap().0
                    == template.name.rsplit_once(':').unwrap().0
                {
                    let context = scope.state();
                    let height = context.height;
                    self.dirty_scopes.insert(DirtyScope {
                        height,
                        id: context.id,
                    });
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
    /// root component will be ran once and then diffed. All updates will flow out as mutations.
    ///
    /// All state stored in components will be completely wiped away.
    ///
    /// Any templates previously registered will remain.
    ///
    /// # Example
    /// ```rust, ignore
    /// static app: Component = |cx|  rsx!{ "hello world" };
    ///
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self, to: &mut impl WriteMutations) {
        self.flush_templates(to);
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        let new_nodes = self.run_scope(ScopeId::ROOT);

        // Rebuilding implies we append the created elements to the root
        let m = self.create_scope(to, ScopeId::ROOT, new_nodes, None);

        to.append_children(ElementId(0), m);
    }

    /// Render whatever the VirtualDom has ready as fast as possible without requiring an executor to progress
    /// suspended subtrees.
    pub fn render_immediate(&mut self, to: &mut impl WriteMutations) {
        self.flush_templates(to);

        // Process any events that might be pending in the queue
        // Signals marked with .write() need a chance to be handled by the effect driver
        // This also processes futures which might progress into immediates
        self.process_events();

        // Next, diff any dirty scopes
        // We choose not to poll the deadline since we complete pretty quickly anyways
        while let Some(dirty) = self.dirty_scopes.pop_first() {
            // If the scope doesn't exist for whatever reason, then we should skip it
            if !self.scopes.contains(dirty.id.0) {
                continue;
            }

            {
                let _runtime = RuntimeGuard::new(self.runtime.clone());
                // Run the scope and get the mutations
                let new_nodes = self.run_scope(dirty.id);

                self.diff_scope(to, dirty.id, new_nodes);
            }
        }
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
    /// however any futures wating on flush_sync will remain pending
    pub async fn wait_for_suspense(&mut self) {
        loop {
            if self.suspended_scopes.is_empty() {
                break;
            }

            // Wait for a work to be ready (IE new suspense leaves to pop up)
            self.poll_tasks().await;

            // Render whatever work needs to be rendered, unlocking new futures and suspense leaves
            self.render_immediate(&mut NoOpMutations);
        }
    }

    /// Get the current runtime
    pub fn runtime(&self) -> Rc<Runtime> {
        self.runtime.clone()
    }

    /// Flush any queued template changes
    fn flush_templates(&mut self, to: &mut impl WriteMutations) {
        for template in self.queued_templates.drain(..) {
            to.register_template(template);
        }
    }

    /*
    ------------------------
    The algorithm works by walking through the list of dynamic attributes, checking their paths, and breaking when
    we find the target path.

    With the target path, we try and move up to the parent until there is no parent.
    Due to how bubbling works, we call the listeners before walking to the parent.

    If we wanted to do capturing, then we would accumulate all the listeners and call them in reverse order.
    ----------------------

    For a visual demonstration, here we present a tree on the left and whether or not a listener is collected on the
    right.

    |           <-- yes (is ascendant)
    | | |       <-- no  (is not direct ascendant)
    | |         <-- yes (is ascendant)
    | | | | |   <--- target element, break early, don't check other listeners
    | | |       <-- no, broke early
    |           <-- no, broke early
    */
    fn handle_bubbling_event(
        &mut self,
        mut parent: Option<ElementRef>,
        name: &str,
        uievent: Event<dyn Any>,
    ) {
        // If the event bubbles, we traverse through the tree until we find the target element.
        // Loop through each dynamic attribute (in a depth first order) in this template before moving up to the template's parent.
        while let Some(path) = parent {
            let mut listeners = vec![];

            let el_ref = &self.mounts[path.mount.0].node;
            let node_template = el_ref.template.get();
            let target_path = path.path;

            // Accumulate listeners into the listener list bottom to top
            for (idx, attrs) in el_ref.dynamic_attrs.iter().enumerate() {
                let this_path = node_template.attr_paths[idx];

                for attr in attrs.iter() {
                    // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                    if attr.name.trim_start_matches("on") == name
                        && target_path.is_decendant(&this_path)
                    {
                        listeners.push(&attr.value);

                        // Break if this is the exact target element.
                        // This means we won't call two listeners with the same name on the same element. This should be
                        // documented, or be rejected from the rsx! macro outright
                        if target_path == this_path {
                            break;
                        }
                    }
                }
            }

            // Now that we've accumulated all the parent attributes for the target element, call them in reverse order
            // We check the bubble state between each call to see if the event has been stopped from bubbling
            for listener in listeners.into_iter().rev() {
                if let AttributeValue::Listener(listener) = listener {
                    self.runtime.rendering.set(false);
                    listener.call(uievent.clone());
                    self.runtime.rendering.set(true);

                    if !uievent.propagates.get() {
                        return;
                    }
                }
            }

            let mount = el_ref.mount.get().as_usize();
            parent = mount.and_then(|id| self.mounts.get(id).and_then(|el| el.parent));
        }
    }

    /// Call an event listener in the simplest way possible without bubbling upwards
    fn handle_non_bubbling_event(&mut self, node: ElementRef, name: &str, uievent: Event<dyn Any>) {
        let el_ref = &self.mounts[node.mount.0].node;
        let node_template = el_ref.template.get();
        let target_path = node.path;

        for (idx, attr) in el_ref.dynamic_attrs.iter().enumerate() {
            let this_path = node_template.attr_paths[idx];

            for attr in attr.iter() {
                // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                // Only call the listener if this is the exact target element.
                if attr.name.trim_start_matches("on") == name && target_path == this_path {
                    if let AttributeValue::Listener(listener) = &attr.value {
                        self.runtime.rendering.set(false);
                        listener.call(uievent.clone());
                        self.runtime.rendering.set(true);
                        break;
                    }
                }
            }
        }
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
