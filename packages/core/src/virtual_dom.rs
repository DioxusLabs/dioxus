//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
/*


We use a three-phase reconciliation process to update the DOM:
- Progress state changes
- Diff the new state with the old state
- Apply the diff to the DOM and commit the changes

Each phase should be idempotent - we should be able to progress state changes all we want and then diff and apply the
changes at any time. Suspense is built on this - components call themselves repeatedly until









*/

use crate::{
    any_props::VProps,
    innerlude::{
        DirtyScope, ElementId, ElementRef, ErrorBoundary, Mutations, Scheduler, SchedulerMsg,
        ScopeId, ScopeSlab, ScopeState,
    },
    mutations::Mutation,
    nodes::{Template, TemplateId},
    scheduler::SuspenseId,
    AttributeValue, DynamicNode, Element, Event, Scope, SuspenseContext, TaskId,
};
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::{pin_mut, StreamExt};
use rustc_hash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::{
    any::Any, borrow::BorrowMut, cell::Cell, collections::BTreeSet, future::Future, rc::Rc,
    task::Context,
};

/// A virtual node system that progresses user events and diffs UI trees.
///
/// ## Guide
///
/// Components are defined as simple functions that take [`Scope`] and return an [`Element`].
///
/// ```rust
/// # use dioxus::prelude::*;
///
/// #[derive(Props, PartialEq)]
/// struct AppProps {
///     title: String
/// }
///
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         div {"hello, {cx.props.title}"}
///     ))
/// }
/// ```
///
/// Components may be composed to make complex apps.
///
/// ```rust
/// # #![allow(unused)]
/// # use dioxus::prelude::*;
///
/// # #[derive(Props, PartialEq)]
/// # struct AppProps {
/// #     title: String
/// # }
///
/// static ROUTES: &str = "";
///
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{cx.props.title}" }
///         Footer {}
///     ))
/// }
///
/// #[inline_props]
/// fn NavBar(cx: Scope, routes: &'static str) -> Element {
///     cx.render(rsx! {
///         div { "Routes: {routes}" }
///     })
/// }
///
/// fn Footer(cx: Scope) -> Element {
///     cx.render(rsx! { div { "Footer" } })
/// }
///
/// #[inline_props]
/// fn Title<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
///     cx.render(rsx! {
///         div { id: "title", children }
///     })
/// }
/// ```
///
/// To start an app, create a [`VirtualDom`] and call [`VirtualDom::rebuild`] to get the list of edits required to
/// draw the UI.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # fn App(cx: Scope) -> Element { cx.render(rsx! { div {} }) }
///
/// let mut vdom = VirtualDom::new(App);
/// let edits = vdom.rebuild();
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
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx! {
///         div { "Hello World" }
///     })
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
    // Maps a template path to a map of byteindexes to templates
    pub(crate) templates: FxHashMap<TemplateId, FxHashMap<usize, Template<'static>>>,
    pub(crate) scopes: ScopeSlab,

    // These are renders we've accumulated from needs_update from within components
    pub(crate) queued_renders: BTreeSet<DirtyScope>,

    // These are dirty scopes that need to be diffed since they were marked as dirty during rendering
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,

    // Every element is actually a dual reference - one to the template and the other to the dynamic node in that template
    pub(crate) elements: Slab<ElementRef>,

    // While diffing we need some sort of way of breaking off a stream of suspended mutations.
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) collected_leaves: Vec<SuspenseId>,

    pub(crate) mutations: Mutations<'static>,

    // Scheduler stuff
    pub(crate) scheduler: Rc<Scheduler>,
    pub(crate) scheduler_rx: UnboundedReceiver<SchedulerMsg>,

    // Suspense stuff
    pub(crate) suspense_roots: FxHashSet<ScopeId>,
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
    /// fn Example(cx: Scope) -> Element  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(app: fn(Scope) -> Element) -> Self {
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
    /// fn Example(cx: Scope<SomeProps>) -> Element  {
    ///     cx.render(rsx!{ div{ "hello {cx.props.name}" } })
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
    pub fn new_with_props<P: 'static>(root: fn(Scope<P>) -> Element, root_props: P) -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        // Create the empty dom, we will populate here in a sec
        let mut dom = Self {
            scheduler_rx: rx,
            scheduler: Scheduler::new(tx),
            templates: Default::default(),
            scopes: Default::default(),
            elements: Default::default(),
            scope_stack: Vec::new(),
            queued_renders: BTreeSet::new(),
            dirty_scopes: BTreeSet::new(),
            collected_leaves: Vec::new(),
            mutations: Mutations::default(),
            suspense_roots: Default::default(),
        };

        // Create the root scope
        // There is no diffing of the props here, hence the unreachable
        let root = dom.new_scope(
            Box::new(VProps::new(root, |_, _| unreachable!(), root_props)),
            "app",
        );

        // Mark this as dirty right away so render it from render_with_deadline
        root.needs_update();

        // The root component is always a suspense boundary for any async children
        // This could be unexpected, so we might rethink this behavior later
        //
        // We *could* just panic if the suspense boundary is not found
        root.provide_context(Rc::new(SuspenseContext::new(ScopeId(0))));

        // the root element is always given element ID 0 since it's the container for the entire tree
        dom.elements.insert(ElementRef::none());

        dom
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get(id)
    }

    /// Get the single scope at the top of the VirtualDom tree that will always be around
    ///
    /// This scope has a ScopeId of 0 and is the root of the tree
    pub fn base_scope(&self) -> &ScopeState {
        self.scopes.get(ScopeId(0)).unwrap()
    }

    /// Build the virtualdom with a global context inserted into the base scope
    ///
    /// This is useful for what is essentially dependency injection when building the app
    pub fn with_root_context<T: Clone + 'static>(self, context: T) -> Self {
        self.base_scope().provide_context(context);
        self
    }

    pub fn queue_render(&mut self, id: ScopeId) {
        if let Some(scope) = self.scopes.get(id) {
            self.queued_renders.insert(DirtyScope {
                height: scope.height,
                id,
            });
        }
    }

    /// Manually mark a scope as requiring a re-render
    ///
    /// Whenever the VirtualDom "works", it will re-render this scope
    pub fn mark_dirty(&mut self, id: ScopeId) {
        if let Some(scope) = self.scopes.get(id) {
            let height = scope.height;
            self.dirty_scopes.insert(DirtyScope { height, id });
        }
    }

    /// Determine whether or not a scope is currently in a suspended state
    ///
    /// This does not mean the scope is waiting on its own futures, just that the tree that the scope exists in is
    /// currently suspended.
    pub fn is_scope_suspended(&self, id: ScopeId) -> bool {
        !self.scopes[id]
            .consume_context::<Rc<SuspenseContext>>()
            .unwrap()
            .waiting_on
            .borrow()
            .is_empty()
    }

    /// Determine if the tree is at all suspended. Used by SSR and other outside mechanisms to determine if the tree is
    /// ready to be rendered.
    pub fn has_suspended_work(&self) -> bool {
        todo!()
        // !self.scheduler.leaves.borrow().is_empty()
    }

    /// Call a listener inside the VirtualDom with data from outside the VirtualDom.
    ///
    /// This method will identify the appropriate element. The data must match up with the listener delcared. Note that
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
        let mut parent_path = self.elements.get(element.0);
        let mut listeners = vec![];

        // We will clone this later. The data itself is wrapped in RC to be used in callbacks if required
        let uievent = Event {
            propagates: Rc::new(Cell::new(bubbles)),
            data,
        };

        // If the event bubbles, we traverse through the tree until we find the target element.
        if bubbles {
            // Loop through each dynamic attribute (in a depth first order) in this template before moving up to the template's parent.
            while let Some(el_ref) = parent_path {
                // safety: we maintain references of all vnodes in the element slab
                if let Some(template) = el_ref.template {
                    let template = unsafe { template.as_ref() };
                    let node_template = template.template.get();
                    let target_path = el_ref.path;

                    for (idx, attr) in template.dynamic_attrs.iter().enumerate() {
                        let this_path = node_template.attr_paths[idx];

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

                    // Now that we've accumulated all the parent attributes for the target element, call them in reverse order
                    // We check the bubble state between each call to see if the event has been stopped from bubbling
                    for listener in listeners.drain(..).rev() {
                        if let AttributeValue::Listener(listener) = listener {
                            if let Some(cb) = listener.borrow_mut().as_deref_mut() {
                                cb(uievent.clone());
                            }

                            if !uievent.propagates.get() {
                                return;
                            }
                        }
                    }

                    parent_path = template.parent.and_then(|id| self.elements.get(id.0));
                } else {
                    break;
                }
            }
        } else {
            // Otherwise, we just call the listener on the target element
            if let Some(el_ref) = parent_path {
                // safety: we maintain references of all vnodes in the element slab
                if let Some(template) = el_ref.template {
                    let template = unsafe { template.as_ref() };
                    let node_template = template.template.get();
                    let target_path = el_ref.path;

                    for (idx, attr) in template.dynamic_attrs.iter().enumerate() {
                        let this_path = node_template.attr_paths[idx];

                        // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                        // Only call the listener if this is the exact target element.
                        if attr.name.trim_start_matches("on") == name && target_path == this_path {
                            if let AttributeValue::Listener(listener) = &attr.value {
                                if let Some(cb) = listener.borrow_mut().as_deref_mut() {
                                    cb(uievent.clone());
                                }

                                break;
                            }
                        }
                    }
                }
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
    /// let dom = VirtualDom::new(App);
    /// let sender = dom.get_scheduler_channel();
    /// ```
    pub async fn wait_for_work(&mut self) {
        let mut some_msg = None;

        loop {
            match some_msg.take() {
                // If a bunch of messages are ready in a sequence, try to pop them off synchronously
                Some(msg) => match msg {
                    // Queue this scope for rendering
                    // Note this won't queue the scope for diffing as it might be suspended
                    SchedulerMsg::Immediate(id) => self.queue_render(id),

                    // Go poll the scheduler
                    SchedulerMsg::TaskNotified(task) => self.handle_task_wakeup(task),
                },

                // If they're not ready, then we should wait for them to be ready
                None => {
                    match self.scheduler_rx.try_next() {
                        Ok(Some(val)) => some_msg = Some(val),
                        Ok(None) => return,
                        Err(_) => {
                            // If we have any dirty scopes, or finished fiber trees then we should exit
                            if !self.dirty_scopes.is_empty() || !self.queued_renders.is_empty() {
                                return;
                            }

                            some_msg = self.scheduler_rx.next().await
                        }
                    }
                }
            }
        }
    }

    /// Process all events in the queue until there are no more left
    pub fn process_events(&mut self) {
        while let Ok(Some(msg)) = self.scheduler_rx.try_next() {
            match msg {
                SchedulerMsg::Immediate(id) => self.queue_render(id),
                SchedulerMsg::TaskNotified(task) => self.handle_task_wakeup(task),
            }
        }
    }

    /// Handle notifications by tasks inside the scheduler
    ///
    /// This is precise, meaning we won't poll every task, just tasks that have woken up as notified to use by the
    /// queue
    fn handle_task_wakeup(&mut self, id: TaskId) {
        let mut tasks = self.scheduler.tasks.borrow_mut();

        let task = match tasks.get(id.0) {
            Some(task) => task,
            // The task was removed from the scheduler, so we can just ignore it
            None => return,
        };

        let mut cx = Context::from_waker(&task.waker);

        // If the task completes...
        if task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            let scope = &self.scopes[task.scope];
            scope.spawned_tasks.borrow_mut().remove(&id);

            // Remove it from the scheduler
            tasks.try_remove(id.0);
        }
    }

    /// Replace a template at runtime. This will re-render all components that use this template.
    /// This is the primitive that enables hot-reloading.
    ///
    /// The caller must ensure that the template refrences the same dynamic attributes and nodes as the original template.
    ///
    /// This will only replace the the parent template, not any nested templates.
    pub fn replace_template(&mut self, template: Template<'static>) {
        self.register_template_first_byte_index(template);
        // iterating a slab is very inefficient, but this is a rare operation that will only happen during development so it's fine
        for scope in self.scopes.iter() {
            todo!()
            // if let Some(RenderReturn::Ready(sync)) = scope.try_root_node() {
            //     if sync.template.get().name.rsplit_once(':').unwrap().0
            //         == template.name.rsplit_once(':').unwrap().0
            //     {
            //         let height = scope.height;
            //         self.dirty_scopes.insert(DirtyScope {
            //             height,
            //             id: scope.id,
            //         });
            //     }
            // }
        }
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
    /// static App: Component = |cx|  cx.render(rsx!{ "hello world" });
    ///
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        todo!()
        // match unsafe { self.run_scope(ScopeId(0)) } {
        //     // Rebuilding implies we append the created elements to the root
        //     RenderReturn::Ready(node) => {
        //         let m = self.create_scope(ScopeId(0), node);
        //         self.mutations.edits.push(Mutation::AppendChildren {
        //             id: ElementId(0),
        //             m,
        //         });
        //     }
        //     // If an error occurs, we should try to render the default error component and context where the error occured
        //     RenderReturn::Aborted(_placeholder) => panic!("Cannot catch errors during rebuild"),
        // }

        // self.finalize()
    }

    /// Render whatever the VirtualDom has ready as fast as possible without requiring an executor to progress
    /// suspended subtrees.
    pub fn render_immediate(&mut self) -> Mutations {
        // Build a waker that won't wake up since our deadline is already expired when it's polled
        let waker = futures_util::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);

        // Now run render with deadline but dont even try to poll any async tasks
        let fut = self.render_with_deadline(std::future::ready(()));
        pin_mut!(fut);

        // The root component is not allowed to be async
        match fut.poll(&mut cx) {
            std::task::Poll::Ready(mutations) => mutations,
            std::task::Poll::Pending => panic!("render_immediate should never return pending"),
        }
    }

    // Compute the diff for a scope
    // Once we hit a memoization barrier, we will stop
    pub fn compute_diff(&mut self, scope: ScopeId) -> Mutations {
        self.diff_scope(scope);
        self.finalize()
    }

    fn queue_dynamic_node(&mut self, node: &DynamicNode) {
        match node {
            DynamicNode::Component(c) => {
                // Check if the props have changed and we need to queue a render for the child

                let id = self.load_scope_from_vcomponent(c);

                c.scope.set(Some(id));

                println!("queueing render for component {:?}", c);
                self.queue_render(id);
            }
            DynamicNode::Fragment(f) => {
                for node in f.iter() {
                    for node in node.dynamic_nodes.iter() {
                        self.queue_dynamic_node(node);
                    }
                }
            }
            DynamicNode::Text(_) => {}
            DynamicNode::Placeholder(_) => {}
        }
    }

    /// Wait for all suspense to be resolved
    ///
    /// When you follow this method up with render_immediate, you can be sure that all suspended components have been resolved
    pub async fn wait_for_suspsnese(&mut self) {
        loop {
            // Poll any events that have occured since the last render
            self.process_events();

            println!("processed events {:#?}", self.queued_renders);

            // First, progress all components that were enqueued
            // Eventually we need recursion protection here
            if let Some(queued) = self.queued_renders.iter().next().cloned() {
                self.queued_renders.remove(&queued);

                // If the scope doesn't exist for whatever reason, then we should skip it
                if !self.scopes.contains(queued.id) {
                    continue;
                }

                // render the component
                // If the component creates new components, those will be added to the queued_renders list
                let els = self.run_scope(queued.id);
                let els: &Element = unsafe { std::mem::transmute(els) };

                if let Some(el) = els.as_ref() {
                    // If the component created new nodes, we should run those
                    for node in el.dynamic_nodes.iter() {
                        self.queue_dynamic_node(node);
                    }
                }
            }

            println!("Waiting for work...");

            // Keep going if there is more work to do
            if !self.queued_renders.is_empty() {
                println!(
                    "Continuing because queued renders: {:#?}",
                    self.queued_renders
                );
                continue;
            }

            // If all suspense is resolved, break
            if self.suspense_roots.is_empty() {
                break;
            }

            println!("Suspense roots: {:#?}", self.suspense_roots);

            // Wait for a wakeup
            self.wait_for_work().await;
        }
    }

    /// Progress state without generating mutations
    ///
    /// Useful in SSR scenarios where you want to progress whatever suspense exists and then render that.
    pub async fn progress(&mut self, deadline: impl Future<Output = ()>) {
        pin_mut!(deadline);

        loop {
            // Poll any events that have occured since the last render
            self.process_events();

            println!("processed events {:#?}", self.queued_renders);

            // First, progress all components that were enqueued
            // Eventually we need recursion protection here
            if let Some(queued) = self.queued_renders.iter().next().cloned() {
                self.queued_renders.remove(&queued);

                // If the scope doesn't exist for whatever reason, then we should skip it
                if !self.scopes.contains(queued.id) {
                    continue;
                }

                // render the component
                // If the component creates new components, those will be added to the queued_renders list
                let els = self.run_scope(queued.id);
            }

            println!("Waiting for work...");

            // Poll futures in the meantime
            let mut work = self.wait_for_work();

            // safety: this is okay since we don't touch the original future
            let pinned = unsafe { std::pin::Pin::new_unchecked(&mut work) };

            // If the deadline is exceded (left) then we should return the mutations we have
            use futures_util::future::{select, Either};
            if let Either::Left((_, _)) = select(&mut deadline, pinned).await {
                println!("deadline reached");
                // release the borrowed
                drop(work);
                return;
            }
        }
    }

    /// Render what you can given the deadline.
    ///
    /// The "deadline" here is *only* used to determine when to stop rendering suspense trees. If you are trying to batch
    /// mutations between frames, you should call "progress" with the deadline, and then call "render_immediate" to get the
    /// immediate mutations. We suggest providing a slightly shorter deadline than your frame time since render_immediate
    /// might take some time
    pub async fn render_with_deadline(&mut self, deadline: impl Future<Output = ()>) -> Mutations {
        pin_mut!(deadline);

        // We're going to loop until we know that:
        // 1. There are no more suspense trees to render
        // 2. There are no more dirty scopes
        // 3. The deadline has been reached
        loop {
            // 1. Progress all state until suspense is finished
            //
            // First, progress all state that makes sense to progress
            // If there are suspense blocks, we need to wait until they're done before we can diff them
            loop {
                // Poll any events that have occured since the last render
                self.process_events();

                // First, progress all components that were enqueued
                // Eventually we need recursion protection here
                if let Some(queued) = self.queued_renders.iter().next().cloned() {
                    self.queued_renders.remove(&queued);

                    // If the scope doesn't exist for whatever reason, then we should skip it
                    if !self.scopes.contains(queued.id) {
                        continue;
                    }

                    // render the component
                    // If the component creates new components, those will be added to the queued_renders list
                    let els = self.run_scope(queued.id);
                }
            }
        }

        // Now that we've progress all components that are enqueued, we want to see if we can diff any components
        // Note that we can't diff components that are suspended
        // We need to wait until those suspense trees are done before we can write them out

        // Write out our mutations
        self.finalize()

        // loop {
        //     // first, unload any complete suspense trees
        //     for finished_fiber in self.finished_fibers.drain(..) {
        //         let scope = &self.scopes[finished_fiber];
        //         let context = scope.has_context::<Rc<SuspenseContext>>().unwrap();

        //         self.mutations
        //             .templates
        //             .append(&mut context.mutations.borrow_mut().templates);

        //         self.mutations
        //             .edits
        //             .append(&mut context.mutations.borrow_mut().edits);

        //         // TODO: count how many nodes are on the stack?
        //         self.mutations.push(Mutation::ReplaceWith {
        //             id: context.placeholder.get().unwrap(),
        //             m: 1,
        //         })
        //     }

        //     // Next, diff any dirty scopes
        //     // We choose not to poll the deadline since we complete pretty quickly anyways
        //     if let Some(dirty) = self.dirty_scopes.iter().next().cloned() {
        //         self.dirty_scopes.remove(&dirty);

        //         // If the scope doesn't exist for whatever reason, then we should skip it
        //         if !self.scopes.contains(dirty.id) {
        //             continue;
        //         }

        //         // if the scope is currently suspended, then we should skip it, ignoring any tasks calling for an update
        //         if self.is_scope_suspended(dirty.id) {
        //             continue;
        //         }

        //         // Save the current mutations length so we can split them into boundary
        //         let mutations_to_this_point = self.mutations.edits.len();

        //         // Run the scope and get the mutations
        //         self.run_scope(dirty.id);
        //         self.diff_scope(dirty.id);

        //         // If suspended leaves are present, then we should find the boundary for this scope and attach things
        //         // No placeholder necessary since this is a diff
        //         if !self.collected_leaves.is_empty() {
        //             let mut boundary = self.scopes[dirty.id]
        //                 .consume_context::<Rc<SuspenseContext>>()
        //                 .unwrap();

        //             let boundary_mut = boundary.borrow_mut();

        //             // Attach mutations
        //             boundary_mut
        //                 .mutations
        //                 .borrow_mut()
        //                 .edits
        //                 .extend(self.mutations.edits.split_off(mutations_to_this_point));

        //             // Attach suspended leaves
        //             boundary
        //                 .waiting_on
        //                 .borrow_mut()
        //                 .extend(self.collected_leaves.drain(..));
        //         }
        //     }

        //     // If there's more work, then just continue, plenty of work to do
        //     if !self.dirty_scopes.is_empty() {
        //         continue;
        //     }

        //     // // If there's no pending suspense, then we have no reason to wait for anything
        //     // if self.scheduler.leaves.borrow().is_empty() {
        //     //     return self.finalize();
        //     // }

        //     // Poll the suspense leaves in the meantime
        //     let mut work = self.wait_for_work();

        //     // safety: this is okay since we don't touch the original future
        //     let pinned = unsafe { std::pin::Pin::new_unchecked(&mut work) };

        //     // If the deadline is exceded (left) then we should return the mutations we have
        //     use futures_util::future::{select, Either};
        //     if let Either::Left((_, _)) = select(&mut deadline, pinned).await {
        //         // release the borrowed
        //         drop(work);
        //         return self.finalize();
        //     }
        // }
    }

    /// Swap the current mutations with a new
    fn finalize(&mut self) -> Mutations {
        std::mem::take(&mut self.mutations)
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // Simply drop this scope which drops all of its children
        // self.drop_scope(ScopeId(0), true);
    }
}
