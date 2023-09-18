//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::{
    any_props::VProps,
    arena::ElementId,
    innerlude::{DirtyScope, ElementRef, ErrorBoundary, Mutations, Scheduler, SchedulerMsg},
    mutations::Mutation,
    nodes::RenderReturn,
    nodes::{Template, TemplateId},
    runtime::{Runtime, RuntimeGuard},
    scopes::{ScopeId, ScopeState},
    AttributeValue, Element, Event, Scope, VNode,
};
use futures_util::{pin_mut, StreamExt};
use rustc_hash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::{any::Any, cell::Cell, collections::BTreeSet, future::Future, ptr::NonNull, rc::Rc};

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
/// #[component]
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{cx.props.title}" }
///         Footer {}
///     ))
/// }
///
/// #[component]
/// fn NavBar(cx: Scope, routes: &'static str) -> Element {
///     cx.render(rsx! {
///         div { "Routes: {routes}" }
///     })
/// }
///
/// #[component]
/// fn Footer(cx: Scope) -> Element {
///     cx.render(rsx! { div { "Footer" } })
/// }
///
/// #[component]
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
/// #[component]
/// fn App(cx: Scope) -> Element {
///     cx.render(rsx! {
///         div { "Hello World" }
///     })
/// }
///
/// let dom = VirtualDom::new(App);
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
    pub(crate) scopes: Slab<Box<ScopeState>>,

    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,

    // Maps a template path to a map of byteindexes to templates
    pub(crate) templates: FxHashMap<TemplateId, FxHashMap<usize, Template<'static>>>,

    // Every element is actually a dual reference - one to the template and the other to the dynamic node in that template
    pub(crate) element_refs: Slab<Option<NonNull<VNode<'static>>>>,

    // The element ids that are used in the renderer
    pub(crate) elements: Slab<Option<ElementRef>>,

    pub(crate) mutations: Mutations<'static>,

    pub(crate) runtime: Rc<Runtime>,

    // Currently suspended scopes
    pub(crate) suspended_scopes: FxHashSet<ScopeId>,

    pub(crate) rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,
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
        let scheduler = Scheduler::new(tx);
        let mut dom = Self {
            rx,
            runtime: Runtime::new(scheduler),
            scopes: Default::default(),
            dirty_scopes: Default::default(),
            templates: Default::default(),
            elements: Default::default(),
            element_refs: Default::default(),
            mutations: Mutations::default(),
            suspended_scopes: Default::default(),
        };

        let root = dom.new_scope(
            Box::new(VProps::new(root, |_, _| unreachable!(), root_props)),
            "app",
        );

        // Unlike react, we provide a default error boundary that just renders the error as a string
        root.provide_context(Rc::new(ErrorBoundary::new(ScopeId::ROOT)));

        // the root element is always given element ID 0 since it's the container for the entire tree
        dom.elements.insert(None);

        dom
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get(id.0).map(|s| &**s)
    }

    /// Get the single scope at the top of the VirtualDom tree that will always be around
    ///
    /// This scope has a ScopeId of 0 and is the root of the tree
    pub fn base_scope(&self) -> &ScopeState {
        self.get_scope(ScopeId::ROOT).unwrap()
    }

    /// Build the virtualdom with a global context inserted into the base scope
    ///
    /// This is useful for what is essentially dependency injection when building the app
    pub fn with_root_context<T: Clone + 'static>(self, context: T) -> Self {
        self.base_scope().provide_context(context);
        self
    }

    /// Manually mark a scope as requiring a re-render
    ///
    /// Whenever the Runtime "works", it will re-render this scope
    pub fn mark_dirty(&mut self, id: ScopeId) {
        if let Some(scope) = self.get_scope(id) {
            let height = scope.height();
            tracing::trace!("Marking scope {:?} ({}) as dirty", id, scope.context().name);
            self.dirty_scopes.insert(DirtyScope { height, id });
        }
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
        let _runtime = RuntimeGuard::new(self.runtime.clone());

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
        let parent_path = match self.elements.get(element.0) {
            Some(Some(el)) => el,
            _ => return,
        };
        let mut parent_node = self
            .element_refs
            .get(parent_path.template.0)
            .cloned()
            .map(|el| (*parent_path, el));
        let mut listeners = vec![];

        // We will clone this later. The data itself is wrapped in RC to be used in callbacks if required
        let uievent = Event {
            propagates: Rc::new(Cell::new(bubbles)),
            data,
        };

        // If the event bubbles, we traverse through the tree until we find the target element.
        if bubbles {
            // Loop through each dynamic attribute (in a depth first order) in this template before moving up to the template's parent.
            while let Some((path, el_ref)) = parent_node {
                // safety: we maintain references of all vnodes in the element slab
                let template = unsafe { el_ref.unwrap().as_ref() };
                let node_template = template.template.get();
                let target_path = path.path;

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
                        let origin = path.scope;
                        self.runtime.scope_stack.borrow_mut().push(origin);
                        self.runtime.rendering.set(false);
                        if let Some(cb) = listener.borrow_mut().as_deref_mut() {
                            cb(uievent.clone());
                        }
                        self.runtime.scope_stack.borrow_mut().pop();
                        self.runtime.rendering.set(true);

                        if !uievent.propagates.get() {
                            return;
                        }
                    }
                }

                parent_node = template.parent.get().and_then(|element_ref| {
                    self.element_refs
                        .get(element_ref.template.0)
                        .cloned()
                        .map(|el| (element_ref, el))
                });
            }
        } else {
            // Otherwise, we just call the listener on the target element
            if let Some((path, el_ref)) = parent_node {
                // safety: we maintain references of all vnodes in the element slab
                let template = unsafe { el_ref.unwrap().as_ref() };
                let node_template = template.template.get();
                let target_path = path.path;

                for (idx, attr) in template.dynamic_attrs.iter().enumerate() {
                    let this_path = node_template.attr_paths[idx];

                    // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                    // Only call the listener if this is the exact target element.
                    if attr.name.trim_start_matches("on") == name && target_path == this_path {
                        if let AttributeValue::Listener(listener) = &attr.value {
                            let origin = path.scope;
                            self.runtime.scope_stack.borrow_mut().push(origin);
                            self.runtime.rendering.set(false);
                            if let Some(cb) = listener.borrow_mut().as_deref_mut() {
                                cb(uievent.clone());
                            }
                            self.runtime.scope_stack.borrow_mut().pop();
                            self.runtime.rendering.set(true);

                            break;
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
                    SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                    SchedulerMsg::TaskNotified(task) => self.handle_task_wakeup(task),
                },

                // If they're not ready, then we should wait for them to be ready
                None => {
                    match self.rx.try_next() {
                        Ok(Some(val)) => some_msg = Some(val),
                        Ok(None) => return,
                        Err(_) => {
                            // If we have any dirty scopes, or finished fiber trees then we should exit
                            if !self.dirty_scopes.is_empty() || !self.suspended_scopes.is_empty() {
                                return;
                            }

                            some_msg = self.rx.next().await
                        }
                    }
                }
            }
        }
    }

    /// Process all events in the queue until there are no more left
    pub fn process_events(&mut self) {
        while let Ok(Some(msg)) = self.rx.try_next() {
            match msg {
                SchedulerMsg::Immediate(id) => self.mark_dirty(id),
                SchedulerMsg::TaskNotified(task) => self.handle_task_wakeup(task),
            }
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
        for (_, scope) in self.scopes.iter() {
            if let Some(RenderReturn::Ready(sync)) = scope.try_root_node() {
                if sync.template.get().name.rsplit_once(':').unwrap().0
                    == template.name.rsplit_once(':').unwrap().0
                {
                    let context = scope.context();
                    let height = context.height;
                    self.dirty_scopes.insert(DirtyScope {
                        height,
                        id: context.id,
                    });
                }
            }
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
        let _runtime = RuntimeGuard::new(self.runtime.clone());
        match unsafe { self.run_scope(ScopeId::ROOT).extend_lifetime_ref() } {
            // Rebuilding implies we append the created elements to the root
            RenderReturn::Ready(node) => {
                let m = self.create_scope(ScopeId::ROOT, node);
                self.mutations.edits.push(Mutation::AppendChildren {
                    id: ElementId(0),
                    m,
                });
            }
            // If an error occurs, we should try to render the default error component and context where the error occured
            RenderReturn::Aborted(placeholder) => {
                tracing::debug!("Ran into suspended or aborted scope during rebuild");
                let id = self.next_element();
                placeholder.id.set(Some(id));
                self.mutations.push(Mutation::CreatePlaceholder { id });
            }
        }

        self.finalize()
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

    /// Render the virtual dom, waiting for all suspense to be finished
    ///
    /// The mutations will be thrown out, so it's best to use this method for things like SSR that have async content
    pub async fn wait_for_suspense(&mut self) {
        loop {
            if self.suspended_scopes.is_empty() {
                return;
            }

            self.wait_for_work().await;

            _ = self.render_immediate();
        }
    }

    /// Render what you can given the timeline and then move on
    ///
    /// It's generally a good idea to put some sort of limit on the suspense process in case a future is having issues.
    ///
    /// If no suspense trees are present
    pub async fn render_with_deadline(&mut self, deadline: impl Future<Output = ()>) -> Mutations {
        pin_mut!(deadline);

        self.process_events();

        loop {
            // Next, diff any dirty scopes
            // We choose not to poll the deadline since we complete pretty quickly anyways
            if let Some(dirty) = self.dirty_scopes.iter().next().cloned() {
                self.dirty_scopes.remove(&dirty);

                // If the scope doesn't exist for whatever reason, then we should skip it
                if !self.scopes.contains(dirty.id.0) {
                    continue;
                }

                {
                    let _runtime = RuntimeGuard::new(self.runtime.clone());
                    // Run the scope and get the mutations
                    self.run_scope(dirty.id);
                    self.diff_scope(dirty.id);
                }
            }

            // If there's more work, then just continue, plenty of work to do
            if !self.dirty_scopes.is_empty() {
                continue;
            }

            // Poll the suspense leaves in the meantime
            let mut work = self.wait_for_work();

            // safety: this is okay since we don't touch the original future
            let pinned = unsafe { std::pin::Pin::new_unchecked(&mut work) };

            // If the deadline is exceded (left) then we should return the mutations we have
            use futures_util::future::{select, Either};
            if let Either::Left((_, _)) = select(&mut deadline, pinned).await {
                // release the borrowed
                drop(work);
                return self.finalize();
            }
        }
    }

    /// Swap the current mutations with a new
    fn finalize(&mut self) -> Mutations {
        std::mem::take(&mut self.mutations)
    }

    /// Get the current runtime
    pub fn runtime(&self) -> Rc<Runtime> {
        self.runtime.clone()
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // Simply drop this scope which drops all of its children
        self.drop_scope(ScopeId::ROOT, true);
    }
}
