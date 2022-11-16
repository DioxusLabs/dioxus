use crate::{
    any_props::VComponentProps,
    arena::ElementId,
    arena::ElementPath,
    diff::DirtyScope,
    factory::RenderReturn,
    innerlude::{Mutations, Scheduler, SchedulerMsg},
    mutations::Mutation,
    nodes::{Template, TemplateId},
    scheduler::{SuspenseBoundary, SuspenseId},
    scopes::{ScopeId, ScopeState},
    Attribute, AttributeValue, Element, EventPriority, Scope, SuspenseContext, UiEvent,
};
use futures_util::{pin_mut, FutureExt, StreamExt};
use slab::Slab;
use std::future::Future;
use std::rc::Rc;
use std::{
    any::Any,
    collections::{BTreeSet, HashMap},
};

/// A virtual node system that progresses user events and diffs UI trees.
///
/// ## Guide
///
/// Components are defined as simple functions that take [`Scope`] and return an [`Element`].
///
/// ```rust, ignore
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
/// ```rust, ignore
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{cx.props.title}" }
///         Footer {}
///     ))
/// }
/// ```
///
/// To start an app, create a [`VirtualDom`] and call [`VirtualDom::rebuild`] to get the list of edits required to
/// draw the UI.
///
/// ```rust, ignore
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
/// ```rust
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
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
/// ```rust
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
/// ```
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
    pub(crate) templates: HashMap<TemplateId, Template<'static>>,
    pub(crate) elements: Slab<ElementPath>,
    pub(crate) scopes: Slab<ScopeState>,
    pub(crate) element_stack: Vec<ElementId>,
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,
    pub(crate) scheduler: Rc<Scheduler>,

    // While diffing we need some sort of way of breaking off a stream of suspended mutations.
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) collected_leaves: Vec<SuspenseId>,

    // Whenever a suspense tree is finished, we push its boundary onto this stack.
    // When "render_with_deadline" is called, we pop the stack and return the mutations
    pub(crate) finished_fibers: Vec<ScopeId>,

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
    pub fn new_with_props<P>(root: fn(Scope<P>) -> Element, root_props: P) -> Self
    where
        P: 'static,
    {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let mut dom = Self {
            rx,
            scheduler: Scheduler::new(tx),
            templates: Default::default(),
            scopes: Slab::default(),
            elements: Default::default(),
            scope_stack: Vec::new(),
            element_stack: vec![ElementId(0)],
            dirty_scopes: BTreeSet::new(),
            collected_leaves: Vec::new(),
            finished_fibers: Vec::new(),
        };

        let root = dom.new_scope(Box::into_raw(Box::new(VComponentProps::new(
            root,
            |_, _| unreachable!(),
            root_props,
        ))));

        // The root component is always a suspense boundary for any async children
        // This could be unexpected, so we might rethink this behavior
        root.provide_context(SuspenseBoundary::new(ScopeId(0)));

        // the root element is always given element 0
        dom.elements.insert(ElementPath::null());

        dom
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get(id.0)
    }

    pub fn base_scope(&self) -> &ScopeState {
        self.scopes.get(0).unwrap()
    }

    fn mark_dirty_scope(&mut self, id: ScopeId) {
        let height = self.scopes[id.0].height;
        self.dirty_scopes.insert(DirtyScope { height, id });
    }

    fn is_scope_suspended(&self, id: ScopeId) -> bool {
        !self.scopes[id.0]
            .consume_context::<SuspenseContext>()
            .unwrap()
            .waiting_on
            .borrow()
            .is_empty()
    }

    /// Returns true if there is any suspended work left to be done.
    pub fn has_suspended_work(&self) -> bool {
        !self.scheduler.leaves.borrow().is_empty()
    }

    /// Call a listener inside the VirtualDom with data from outside the VirtualDom.
    ///
    /// This method will identify the appropriate element
    ///
    ///
    ///
    ///
    ///
    ///
    ///
    pub fn handle_event<T: 'static>(
        &mut self,
        name: &str,
        data: Rc<T>,
        element: ElementId,
        bubbles: bool,
        priority: EventPriority,
    ) -> Option<()> {
        /*
        - click registers
        - walk upwards until first element with onclick listener
        - get template from ElementID
        - break out of wait loop
        - send event to virtualdom
        */

        let event = UiEvent {
            bubble_state: std::cell::Cell::new(true),
            data,
        };

        let path = &self.elements[element.0];
        let template = unsafe { &*path.template };
        let dynamic = &template.dynamic_nodes[path.element];

        let location = template
            .dynamic_attrs
            .iter()
            .position(|attr| attr.mounted_element.get() == element)?;

        // let mut index = Some((path.template, location));

        let mut listeners = Vec::<&Attribute>::new();

        // while let Some((raw_parent, dyn_index)) = index {
        //     let parent = unsafe { &mut *raw_parent };
        //     let path = parent.template.node_paths[dyn_index];

        //     listeners.extend(
        //         parent
        //             .dynamic_attrs
        //             .iter()
        //             .enumerate()
        //             .filter_map(|(idx, attr)| {
        //                 match is_path_ascendant(parent.template.node_paths[idx], path) {
        //                     true if attr.name == event.name => Some(attr),
        //                     _ => None,
        //                 }
        //             }),
        //     );

        //     index = parent.parent;
        // }

        for listener in listeners {
            if let AttributeValue::Listener(listener) = &listener.value {
                (listener.borrow_mut())(&event.clone())
            }
        }

        Some(())
    }

    /// Wait for the scheduler to have any work.
    ///
    /// This method polls the internal future queue, waiting for suspense nodes, tasks, or other work. This completes when
    /// any work is ready. If multiple scopes are marked dirty from a task or a suspense tree is finished, this method
    /// will exit.
    ///
    /// This method is cancel-safe, so you're fine to discard the future in a select block.
    ///
    /// This lets us poll async tasks during idle periods without blocking the main thread.
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
                    SchedulerMsg::Immediate(id) => self.mark_dirty_scope(id),
                    SchedulerMsg::TaskNotified(task) => self.handle_task_wakeup(task),
                    SchedulerMsg::SuspenseNotified(id) => self.handle_suspense_wakeup(id),
                },

                // If they're not ready, then we should wait for them to be ready
                None => {
                    match self.rx.try_next() {
                        Ok(Some(val)) => some_msg = Some(val),
                        Ok(None) => return,
                        Err(_) => {
                            // If we have any dirty scopes, or finished fiber trees then we should exit
                            if !self.dirty_scopes.is_empty() || !self.finished_fibers.is_empty() {
                                return;
                            }

                            some_msg = self.rx.next().await
                        }
                    }
                }
            }
        }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom from scratch.
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application.
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
    pub fn rebuild<'a>(&'a mut self) -> Mutations<'a> {
        let mut mutations = Mutations::new(0);

        let root_node = unsafe { self.run_scope_extend(ScopeId(0)) };
        match root_node {
            RenderReturn::Sync(Some(node)) => {
                let m = self.create_scope(ScopeId(0), &mut mutations, node);
                mutations.push(Mutation::AppendChildren { m });
            }
            RenderReturn::Sync(None) => {}
            RenderReturn::Async(_) => unreachable!("Root scope cannot be an async component"),
        }

        mutations
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
        match fut.poll_unpin(&mut cx) {
            std::task::Poll::Ready(mutations) => mutations,
            std::task::Poll::Pending => panic!("render_immediate should never return pending"),
        }
    }

    /// Render what you can given the timeline and then move on
    ///
    /// It's generally a good idea to put some sort of limit on the suspense process in case a future is having issues.
    ///
    /// If no suspense trees are present
    pub async fn render_with_deadline<'a>(
        &'a mut self,
        deadline: impl Future<Output = ()>,
    ) -> Mutations<'a> {
        use futures_util::future::{select, Either};

        let mut mutations = Mutations::new(0);
        pin_mut!(deadline);

        loop {
            // first, unload any complete suspense trees
            for finished_fiber in self.finished_fibers.drain(..) {
                let scope = &mut self.scopes[finished_fiber.0];
                let context = scope.has_context::<SuspenseContext>().unwrap();
                println!("unloading suspense tree {:?}", context.mutations);

                mutations.extend(context.mutations.borrow_mut().template_mutations.drain(..));
                mutations.extend(context.mutations.borrow_mut().drain(..));

                mutations.push(Mutation::ReplaceWith {
                    id: context.placeholder.get().unwrap(),
                    m: 1,
                })
            }

            // Next, diff any dirty scopes
            // We choose not to poll the deadline since we complete pretty quickly anyways
            if let Some(dirty) = self.dirty_scopes.iter().next().cloned() {
                self.dirty_scopes.remove(&dirty);

                // if the scope is currently suspended, then we should skip it, ignoring any tasks calling for an update
                if !self.is_scope_suspended(dirty.id) {
                    self.run_scope(dirty.id);
                    self.diff_scope(&mut mutations, dirty.id);
                }
            }

            // Wait for suspense, or a deadline
            if self.dirty_scopes.is_empty() {
                if self.scheduler.leaves.borrow().is_empty() {
                    return mutations;
                }

                let (work, deadline) = (self.wait_for_work(), &mut deadline);
                pin_mut!(work);

                if let Either::Left((_, _)) = select(deadline, work).await {
                    return mutations;
                }
            }
        }
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // self.drop_scope(ScopeId(0));
    }
}
