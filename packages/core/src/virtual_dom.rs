//! # VirtualDom Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Future, StreamExt};
use fxhash::FxHashSet;
use indexmap::IndexSet;
use std::{any::Any, collections::VecDeque, iter::FromIterator, pin::Pin, sync::Arc, task::Poll};

/// A virtual node s ystem that progresses user events and diffs UI trees.
///
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
/// To inject UserEvents into the VirtualDom, call [`VirtualDom::get_scheduler_channel`] to get access to the scheduler.
///
/// ```rust, ignore
/// let channel = vdom.get_scheduler_channel();
/// channel.send_unbounded(SchedulerMsg::UserEvent(UserEvent {
///     // ...
/// }))
/// ```
///
/// While waiting for UserEvents to occur, call [`VirtualDom::wait_for_work`] to poll any futures inside the VirtualDom.
///
/// ```rust, ignore
/// vdom.wait_for_work().await;
/// ```
///
/// Once work is ready, call [`VirtualDom::work_with_deadline`] to compute the differences between the previous and
/// current UI trees. This will return a [`Mutations`] object that contains Edits, Effects, and NodeRefs that need to be
/// handled by the renderer.
///
/// ```rust, ignore
/// let mutations = vdom.work_with_deadline(|| false);
/// for edit in mutations {
///     apply(edit);
/// }
/// ```
///
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
///
/// ```rust, ignore
/// fn App(cx: Scope<()>) -> Element {
///     cx.render(rsx!{
///         div { "Hello World" }
///     })
/// }
///
/// async fn main() {
///     let mut dom = VirtualDom::new(App);
///
///     let mut inital_edits = dom.rebuild();
///     apply_edits(inital_edits);
///
///     loop {
///         dom.wait_for_work().await;
///         let frame_timeout = TimeoutFuture::new(Duration::from_millis(16));
///         let deadline = || (&mut frame_timeout).now_or_never();
///         let edits = dom.run_with_deadline(deadline).await;
///         apply_edits(edits);
///     }
/// }
/// ```
pub struct VirtualDom {
    scopes: ScopeArena,

    pending_messages: VecDeque<SchedulerMsg>,
    dirty_scopes: IndexSet<ScopeId>,

    channel: (
        UnboundedSender<SchedulerMsg>,
        UnboundedReceiver<SchedulerMsg>,
    ),
}

// Methods to create the VirtualDom
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
    /// fn Example(cx: Scope<()>) -> Element  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(root: Component<()>) -> Self {
        Self::new_with_props(root, ())
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
    pub fn new_with_props<P>(root: Component<P>, root_props: P) -> Self
    where
        P: 'static,
    {
        Self::new_with_props_and_scheduler(
            root,
            root_props,
            futures_channel::mpsc::unbounded::<SchedulerMsg>(),
        )
    }

    /// Launch the VirtualDom, but provide your own channel for receiving and sending messages into the scheduler
    ///
    /// This is useful when the VirtualDom must be driven from outside a thread and it doesn't make sense to wait for the
    /// VirtualDom to be created just to retrieve its channel receiver.
    ///
    /// ```rust
    /// let channel = futures_channel::mpsc::unbounded();
    /// let dom = VirtualDom::new_with_scheduler(Example, (), channel);
    /// ```
    pub fn new_with_props_and_scheduler<P: 'static>(
        root: Component<P>,
        root_props: P,
        channel: (
            UnboundedSender<SchedulerMsg>,
            UnboundedReceiver<SchedulerMsg>,
        ),
    ) -> Self {
        let scopes = ScopeArena::new(channel.0.clone());

        scopes.new_with_key(
            root as *const _,
            Box::new(VComponentProps {
                props: root_props,
                memo: |_a, _b| unreachable!("memo on root will neve be run"),
                render_fn: root,
            }),
            None,
            ElementId(0),
            0,
        );

        Self {
            scopes,
            channel,
            dirty_scopes: IndexSet::from_iter([ScopeId(0)]),
            pending_messages: VecDeque::new(),
        }
    }

    /// Get the [`Scope`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or alternsative renderers that use Dioxus
    /// directly.
    ///
    /// This method is equivalent to calling `get_scope(ScopeId(0))`
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(example);
    /// dom.rebuild();
    ///
    ///
    /// ```
    pub fn base_scope(&self) -> &ScopeState {
        self.get_scope(ScopeId(0)).unwrap()
    }

    /// Get the [`ScopeState`] for a component given its [`ScopeId`]
    ///
    /// # Example
    ///
    ///
    ///
    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get_scope(id)
    }

    /// Get an [`UnboundedSender`] handle to the channel used by the scheduler.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let dom = VirtualDom::new(App);
    /// let sender = dom.get_scheduler_channel();
    /// ```
    pub fn get_scheduler_channel(&self) -> UnboundedSender<SchedulerMsg> {
        self.channel.0.clone()
    }

    /// Add a new message to the scheduler queue directly.
    ///
    ///
    /// This method makes it possible to send messages to the scheduler from outside the VirtualDom without having to
    /// call `get_schedule_channel` and then `send`.
    ///
    /// # Example
    /// ```rust, ignore
    /// let dom = VirtualDom::new(App);
    /// dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    /// ```
    pub fn handle_message(&mut self, msg: SchedulerMsg) {
        if self.channel.0.unbounded_send(msg).is_ok() {
            self.process_all_messages();
        }
    }

    /// Check if the [`VirtualDom`] has any pending updates or work to be done.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let dom = VirtualDom::new(App);
    ///
    /// // the dom is "dirty" when it is started and must be rebuilt to get the first render
    /// assert!(dom.has_any_work());
    /// ```
    pub fn has_work(&self) -> bool {
        !(self.dirty_scopes.is_empty() && self.pending_messages.is_empty())
    }

    /// Wait for the scheduler to have any work.
    ///
    /// This method polls the internal future queue *and* the scheduler channel.
    /// To add work to the VirtualDom, insert a message via the scheduler channel.
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
        loop {
            if !self.dirty_scopes.is_empty() && self.pending_messages.is_empty() {
                break;
            }

            if self.pending_messages.is_empty() {
                if self.scopes.tasks.has_tasks() {
                    use futures_util::future::{select, Either};

                    match select(PollTasks(&mut self.scopes), self.channel.1.next()).await {
                        Either::Left((_, _)) => {}
                        Either::Right((msg, _)) => self.pending_messages.push_front(msg.unwrap()),
                    }
                } else {
                    self.pending_messages
                        .push_front(self.channel.1.next().await.unwrap());
                }
            }

            // Move all the messages into the queue
            self.process_all_messages();
        }
    }

    /// Manually kick the VirtualDom to process any
    pub fn process_all_messages(&mut self) {
        // clear out the scheduler queue
        while let Ok(Some(msg)) = self.channel.1.try_next() {
            self.pending_messages.push_front(msg);
        }

        // process all the messages pulled from the queue
        while let Some(msg) = self.pending_messages.pop_back() {
            self.process_message(msg);
        }
    }

    pub fn process_message(&mut self, msg: SchedulerMsg) {
        match msg {
            SchedulerMsg::NewTask(_id) => {
                // uh, not sure? I think end up re-polling it anyways
            }
            SchedulerMsg::Event(event) => {
                if let Some(element) = event.element {
                    self.scopes.call_listener_with_bubbling(event, element);
                }
            }
            SchedulerMsg::Immediate(s) => {
                self.dirty_scopes.insert(s);
            }
        }
    }

    /// Run the virtualdom with a deadline.
    ///
    /// This method will perform any outstanding diffing work and try to return as many mutations as possible before the
    /// deadline is reached. This method accepts a closure that returns `true` if the deadline has been reached. To wrap
    /// your future into a deadline, consider the `now_or_never` method from `future_utils`.
    ///
    /// ```rust, ignore
    /// let mut vdom = VirtualDom::new(App);
    ///
    /// let timeout = TimeoutFuture::from_ms(16);
    /// let deadline = || (&mut timeout).now_or_never();
    ///
    /// let mutations = vdom.work_with_deadline(deadline);
    /// ```
    ///
    /// This method is useful when needing to schedule the virtualdom around other tasks on the main thread to prevent
    /// "jank". It will try to finish whatever work it has by the deadline to free up time for other work.
    ///
    /// If the work is not finished by the deadline, Dioxus will store it for later and return when work_with_deadline
    /// is called again. This means you can ensure some level of free time on the VirtualDom's thread during the work phase.
    ///
    /// For use in the web, it is expected that this method will be called to be executed during "idle times" and the
    /// mutations to be applied during the "paint times" IE "animation frames". With this strategy, it is possible to craft
    /// entirely jank-free applications that perform a ton of work.
    ///
    /// In general use, Dioxus is plenty fast enough to not need to worry about this.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// fn App(cx: Scope<()>) -> Element {
    ///     cx.render(rsx!( div {"hello"} ))
    /// }
    ///
    /// let mut dom = VirtualDom::new(App);
    ///
    /// loop {
    ///     let mut timeout = TimeoutFuture::from_ms(16);
    ///     let deadline = move || (&mut timeout).now_or_never();
    ///
    ///     let mutations = dom.run_with_deadline(deadline).await;
    ///
    ///     apply_mutations(mutations);
    /// }
    /// ```
    pub fn work_with_deadline(&mut self, mut deadline: impl FnMut() -> bool) -> Vec<Mutations> {
        let mut committed_mutations = vec![];

        while !self.dirty_scopes.is_empty() {
            let scopes = &self.scopes;
            let mut diff_state = DiffState::new(scopes);

            let mut ran_scopes = FxHashSet::default();

            // Sort the scopes by height. Theoretically, we'll de-duplicate scopes by height
            self.dirty_scopes
                .retain(|id| scopes.get_scope(*id).is_some());

            self.dirty_scopes.sort_by(|a, b| {
                let h1 = scopes.get_scope(*a).unwrap().height;
                let h2 = scopes.get_scope(*b).unwrap().height;
                h1.cmp(&h2).reverse()
            });

            if let Some(scopeid) = self.dirty_scopes.pop() {
                if !ran_scopes.contains(&scopeid) {
                    ran_scopes.insert(scopeid);

                    self.scopes.run_scope(scopeid);

                    let (old, new) = (self.scopes.wip_head(scopeid), self.scopes.fin_head(scopeid));
                    diff_state.stack.push(DiffInstruction::Diff { new, old });
                    diff_state.stack.scope_stack.push(scopeid);

                    let scope = scopes.get_scope(scopeid).unwrap();
                    diff_state.stack.element_stack.push(scope.container);
                }
            }

            if diff_state.work(&mut deadline) {
                let DiffState { mutations, .. } = diff_state;

                for scope in &mutations.dirty_scopes {
                    self.dirty_scopes.remove(scope);
                }

                committed_mutations.push(mutations);
            } else {
                // leave the work in an incomplete state
                //
                // todo: we should store the edits and re-apply them later
                // for now, we just dump the work completely (threadsafe)
                return committed_mutations;
            }
        }

        committed_mutations
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
    /// # Example
    /// ```rust, ignore
    /// static App: Component<()> = |cx, props| cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        let scope_id = ScopeId(0);
        let mut diff_state = DiffState::new(&self.scopes);

        self.scopes.run_scope(scope_id);
        diff_state
            .stack
            .create_node(self.scopes.fin_head(scope_id), MountType::Append);

        diff_state.stack.element_stack.push(ElementId(0));
        diff_state.stack.scope_stack.push(scope_id);
        diff_state.work(|| false);
        diff_state.mutations
    }

    /// Compute a manual diff of the VirtualDom between states.
    ///
    /// This can be useful when state inside the DOM is remotely changed from the outside, but not propagated as an event.
    ///
    /// In this case, every component will be diffed, even if their props are memoized. This method is intended to be used
    /// to force an update of the DOM when the state of the app is changed outside of the app.
    ///
    /// To force a reflow of the entire VirtualDom, use `ScopeId(0)` as the scope_id.
    ///
    /// # Example
    /// ```rust, ignore
    /// #[derive(PartialEq, Props)]
    /// struct AppProps {
    ///     value: Shared<&'static str>,
    /// }
    ///
    /// static App: Component<AppProps> = |cx, props|{
    ///     let val = cx.value.borrow();
    ///     cx.render(rsx! { div { "{val}" } })
    /// };
    ///
    /// let value = Rc::new(RefCell::new("Hello"));
    /// let mut dom = VirtualDom::new_with_props(App, AppProps { value: value.clone(), });
    ///
    /// let _ = dom.rebuild();
    ///
    /// *value.borrow_mut() = "goodbye";
    ///
    /// let edits = dom.diff();
    /// ```
    pub fn hard_diff(&mut self, scope_id: ScopeId) -> Mutations {
        let mut diff_machine = DiffState::new(&self.scopes);
        self.scopes.run_scope(scope_id);

        let (old, new) = (
            diff_machine.scopes.wip_head(scope_id),
            diff_machine.scopes.fin_head(scope_id),
        );

        diff_machine.force_diff = true;
        diff_machine.stack.push(DiffInstruction::Diff { old, new });
        diff_machine.stack.scope_stack.push(scope_id);

        let scope = diff_machine.scopes.get_scope(scope_id).unwrap();
        diff_machine.stack.element_stack.push(scope.container);
        diff_machine.work(|| false);

        diff_machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    /// ```rust
    /// fn Base(cx: Scope<()>) -> Element {
    ///     rsx!(cx, div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn render_vnodes<'a>(&'a self, lazy_nodes: LazyNodes<'a, '_>) -> &'a VNode<'a> {
        let scope = self.scopes.get_scope(ScopeId(0)).unwrap();
        let frame = scope.wip_frame();
        let factory = NodeFactory { bump: &frame.bump };
        let node = lazy_nodes.call(factory);
        frame.bump.alloc(node)
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    /// ```rust
    /// fn Base(cx: Scope<()>) -> Element {
    ///     rsx!(cx, div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```   
    pub fn diff_vnodes<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Mutations<'a> {
        let mut machine = DiffState::new(&self.scopes);
        machine.stack.push(DiffInstruction::Diff { new, old });
        machine.stack.element_stack.push(ElementId(0));
        machine.stack.scope_stack.push(ScopeId(0));
        machine.work(|| false);
        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    ///
    /// ```rust
    /// fn Base(cx: Scope<()>) -> Element {
    ///     rsx!(cx, div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn create_vnodes<'a>(&'a self, nodes: LazyNodes<'a, '_>) -> Mutations<'a> {
        let mut machine = DiffState::new(&self.scopes);
        machine.stack.element_stack.push(ElementId(0));
        machine
            .stack
            .create_node(self.render_vnodes(nodes), MountType::Append);
        machine.work(|| false);
        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scopes's arena.
    ///
    /// Useful when needing to diff two rsx! calls from outside the VirtualDom, such as in a test.
    ///
    ///
    /// ```rust
    /// fn Base(cx: Scope<()>) -> Element {
    ///     rsx!(cx, div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn diff_lazynodes<'a>(
        &'a self,
        left: LazyNodes<'a, '_>,
        right: LazyNodes<'a, '_>,
    ) -> (Mutations<'a>, Mutations<'a>) {
        let (old, new) = (self.render_vnodes(left), self.render_vnodes(right));

        let mut create = DiffState::new(&self.scopes);
        create.stack.scope_stack.push(ScopeId(0));
        create.stack.element_stack.push(ElementId(0));
        create.stack.create_node(old, MountType::Append);
        create.work(|| false);

        let mut edit = DiffState::new(&self.scopes);
        edit.stack.scope_stack.push(ScopeId(0));
        edit.stack.element_stack.push(ElementId(0));
        edit.stack.push(DiffInstruction::Diff { old, new });
        edit.work(|| false);

        (create.mutations, edit.mutations)
    }
}

/*
Scopes and ScopeArenas are never dropped internally.
An app will always occupy as much memory as its biggest form.

This means we need to handle all specifics of drop *here*. It's easier
to reason about centralizing all the drop logic in one spot rather than scattered in each module.

Broadly speaking, we want to use the remove_nodes method to clean up *everything*
This will drop listeners, borrowed props, and hooks for all components.
We need to do this in the correct order - nodes at the very bottom must be dropped first to release
the borrow chain.

Once the contents of the tree have been cleaned up, we can finally clean up the
memory used by ScopeState itself.

questions:
should we build a vcomponent for the root?
- probably - yes?
- store the vcomponent in the root dom

- 1: Use remove_nodes to use the ensure_drop_safety pathway to safely drop the tree
- 2: Drop the ScopeState itself
*/
impl Drop for VirtualDom {
    fn drop(&mut self) {
        // the best way to drop the dom is to replace the root scope with a dud
        // the diff infrastructure will then finish the rest
        let scope = self.scopes.get_scope(ScopeId(0)).unwrap();

        // todo: move the remove nodes method onto scopearena
        // this will clear *all* scopes *except* the root scope
        let mut machine = DiffState::new(&self.scopes);
        machine.remove_nodes([scope.root_node()], false);

        // Now, clean up the root scope
        // safety: there are no more references to the root scope
        let scope = unsafe { &mut *self.scopes.get_scope_raw(ScopeId(0)).unwrap() };
        scope.reset();

        // make sure there are no "live" components
        for (_, scopeptr) in self.scopes.scopes.get_mut().drain() {
            // safety: all scopes were made in the bump's allocator
            // They are never dropped until now. The only way to drop is through Box.
            let scope = unsafe { bumpalo::boxed::Box::from_raw(scopeptr) };
            drop(scope);
        }

        for scopeptr in self.scopes.free_scopes.get_mut().drain(..) {
            // safety: all scopes were made in the bump's allocator
            // They are never dropped until now. The only way to drop is through Box.
            let mut scope = unsafe { bumpalo::boxed::Box::from_raw(scopeptr) };
            scope.reset();
            drop(scope);
        }
    }
}

struct PollTasks<'a>(&'a mut ScopeArena);

impl<'a> Future for PollTasks<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut any_pending = false;

        let mut tasks = self.0.tasks.tasks.borrow_mut();
        let mut to_remove = vec![];

        // this would be better served by retain
        for (id, task) in tasks.iter_mut() {
            if task.as_mut().poll(cx).is_ready() {
                to_remove.push(*id);
            } else {
                any_pending = true;
            }
        }

        for id in to_remove {
            tasks.remove(&id);
        }

        // Resolve the future if any singular task is ready
        match any_pending {
            true => Poll::Pending,
            false => Poll::Ready(()),
        }
    }
}

#[derive(Debug)]
pub enum SchedulerMsg {
    // events from the host
    Event(UserEvent),

    // setstate
    Immediate(ScopeId),

    // an async task pushed from an event handler (or just spawned)
    NewTask(ScopeId),
}

/// User Events are events that are shuttled from the renderer into the VirtualDom trhough the scheduler channel.
///
/// These events will be passed to the appropriate Element given by `mounted_dom_id` and then bubbled up through the tree
/// where each listener is checked and fired if the event name matches.
///
/// It is the expectation that the event name matches the corresponding event listener, otherwise Dioxus will panic in
/// attempting to downcast the event data.
///
/// Because Event Data is sent across threads, it must be `Send + Sync`. We are hoping to lift the `Sync` restriction but
/// `Send` will not be lifted. The entire `UserEvent` must also be `Send + Sync` due to its use in the scheduler channel.
///
/// # Example
/// ```rust
/// fn App(cx: Scope<()>) -> Element {
///     rsx!(cx, div {
///         onclick: move |_| println!("Clicked!")
///     })
/// }
///
/// let mut dom = VirtualDom::new(App);
/// let mut scheduler = dom.get_scheduler_channel();
/// scheduler.unbounded_send(SchedulerMsg::UiEvent(
///     UserEvent {
///         scope_id: None,
///         priority: EventPriority::Medium,
///         name: "click",
///         element: Some(ElementId(0)),
///         data: Arc::new(ClickEvent { .. })
///     }
/// )).unwrap();
/// ```
#[derive(Debug)]
pub struct UserEvent {
    /// The originator of the event trigger if available
    pub scope_id: Option<ScopeId>,

    /// The priority of the event to be scheduled around ongoing work
    pub priority: EventPriority,

    /// The optional real node associated with the trigger
    pub element: Option<ElementId>,

    /// The event type IE "onclick" or "onmouseover"
    pub name: &'static str,

    /// The event data to be passed onto the event handler
    pub data: Arc<dyn Any + Send + Sync>,
}

/// Priority of Event Triggers.
///
/// Internally, Dioxus will abort work that's taking too long if new, more important work arrives. Unlike React, Dioxus
/// won't be afraid to pause work or flush changes to the RealDOM. This is called "cooperative scheduling". Some Renderers
/// implement this form of scheduling internally, however Dioxus will perform its own scheduling as well.
///
/// The ultimate goal of the scheduler is to manage latency of changes, prioritizing "flashier" changes over "subtler" changes.
///
/// React has a 5-tier priority system. However, they break things into "Continuous" and "Discrete" priority. For now,
/// we keep it simple, and just use a 3-tier priority system.
///
/// - NoPriority = 0
/// - LowPriority = 1
/// - NormalPriority = 2
/// - UserBlocking = 3
/// - HighPriority = 4
/// - ImmediatePriority = 5
///
/// We still have a concept of discrete vs continuous though - discrete events won't be batched, but continuous events will.
/// This means that multiple "scroll" events will be processed in a single frame, but multiple "click" events will be
/// flushed before proceeding. Multiple discrete events is highly unlikely, though.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum EventPriority {
    /// Work that must be completed during the EventHandler phase.
    ///
    /// Currently this is reserved for controlled inputs.
    Immediate = 3,

    /// "High Priority" work will not interrupt other high priority work, but will interrupt medium and low priority work.
    ///
    /// This is typically reserved for things like user interaction.
    ///
    /// React calls these "discrete" events, but with an extra category of "user-blocking" (Immediate).
    High = 2,

    /// "Medium priority" work is generated by page events not triggered by the user. These types of events are less important
    /// than "High Priority" events and will take precedence over low priority events.
    ///
    /// This is typically reserved for VirtualEvents that are not related to keyboard or mouse input.
    ///
    /// React calls these "continuous" events (e.g. mouse move, mouse wheel, touch move, etc).
    Medium = 1,

    /// "Low Priority" work will always be preempted unless the work is significantly delayed, in which case it will be
    /// advanced to the front of the work queue until completed.
    ///
    /// The primary user of Low Priority work is the asynchronous work system (Suspense).
    ///
    /// This is considered "idle" work or "background" work.
    Low = 0,
}

pub struct FragmentProps<'a>(Element<'a>);
pub struct FragmentBuilder<'a, const BUILT: bool>(Element<'a>);
impl<'a> FragmentBuilder<'a, false> {
    pub fn children(self, children: Element<'a>) -> FragmentBuilder<'a, true> {
        FragmentBuilder(children)
    }
}
impl<'a, const A: bool> FragmentBuilder<'a, A> {
    pub fn build(self) -> FragmentProps<'a> {
        FragmentProps(self.0)
    }
}

/// Access the children elements passed into the component
///
/// This enables patterns where a component is passed children from its parent.
///
/// ## Details
///
/// Unlike React, Dioxus allows *only* lists of children to be passed from parent to child - not arbitrary functions
/// or classes. If you want to generate nodes instead of accepting them as a list, consider declaring a closure
/// on the props that takes Context.
///
/// If a parent passes children into a component, the child will always re-render when the parent re-renders. In other
/// words, a component cannot be automatically memoized if it borrows nodes from its parent, even if the component's
/// props are valid for the static lifetime.
///
/// ## Example
///
/// ```rust, ignore
/// fn App(cx: Scope<()>) -> Element {
///     cx.render(rsx!{
///         CustomCard {
///             h1 {}2
///             p {}
///         }
///     })
/// }
///
/// #[derive(PartialEq, Props)]
/// struct CardProps {
///     children: Element
/// }
///
/// fn CustomCard(cx: Scope<CardProps>) -> Element {
///     cx.render(rsx!{
///         div {
///             h1 {"Title card"}
///             {cx.props.children}
///         }
///     })
/// }
/// ```
impl<'a> Properties for FragmentProps<'a> {
    type Builder = FragmentBuilder<'a, false>;
    const IS_STATIC: bool = false;
    fn builder() -> Self::Builder {
        FragmentBuilder(None)
    }
    unsafe fn memoize(&self, _other: &Self) -> bool {
        false
    }
}

/// Create inline fragments using Component syntax.
///
/// ## Details
///
/// Fragments capture a series of children without rendering extra nodes.
///
/// Creating fragments explicitly with the Fragment component is particularly useful when rendering lists or tables and
/// a key is needed to identify each item.
///
/// ## Example
///
/// ```rust, ignore
/// rsx!{
///     Fragment { key: "abc" }
/// }
/// ```
///
/// ## Usage
///
/// Fragments are incredibly useful when necessary, but *do* add cost in the diffing phase.
/// Try to avoid highly nested fragments if you can. Unlike React, there is no protection against infinitely nested fragments.
///
/// This function defines a dedicated `Fragment` component that can be used to create inline fragments in the RSX macro.
///
/// You want to use this free-function when your fragment needs a key and simply returning multiple nodes from rsx! won't cut it.
#[allow(non_upper_case_globals, non_snake_case)]
pub fn Fragment<'a>(cx: Scope<'a, FragmentProps<'a>>) -> Element {
    let i = cx.props.0.as_ref().map(|f| f.decouple());
    cx.render(LazyNodes::new(|f| f.fragment_from_iter(i)))
}

/// Every "Props" used for a component must implement the `Properties` trait. This trait gives some hints to Dioxus
/// on how to memoize the props and some additional optimizations that can be made. We strongly encourage using the
/// derive macro to implement the `Properties` trait automatically as guarantee that your memoization strategy is safe.
///
/// If your props are 'static, then Dioxus will require that they also be PartialEq for the derived memoize strategy. However,
/// if your props borrow data, then the memoization strategy will simply default to "false" and the PartialEq will be ignored.
/// This tends to be useful when props borrow something that simply cannot be compared (IE a reference to a closure);
///
/// By default, the memoization strategy is very conservative, but can be tuned to be more aggressive manually. However,
/// this is only safe if the props are 'static - otherwise you might borrow references after-free.
///
/// We strongly suggest that any changes to memoization be done at the "PartialEq" level for 'static props. Additionally,
/// we advise the use of smart pointers in cases where memoization is important.
///
/// ## Example
///
/// For props that are 'static:
/// ```rust, ignore ignore
/// #[derive(Props, PartialEq)]
/// struct MyProps {
///     data: String
/// }
/// ```
///
/// For props that borrow:
///
/// ```rust, ignore ignore
/// #[derive(Props)]
/// struct MyProps<'a >{
///     data: &'a str
/// }
/// ```
pub trait Properties: Sized {
    type Builder;
    const IS_STATIC: bool;
    fn builder() -> Self::Builder;

    /// Memoization can only happen if the props are valid for the 'static lifetime
    ///
    /// # Safety
    /// The user must know if their props are static, but if they make a mistake, UB happens
    /// Therefore it's unsafe to memoize.
    unsafe fn memoize(&self, other: &Self) -> bool;
}

impl Properties for () {
    type Builder = EmptyBuilder;
    const IS_STATIC: bool = true;
    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
    unsafe fn memoize(&self, _other: &Self) -> bool {
        true
    }
}
// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) {}
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<'a, T: Properties + 'a>(_: fn(Scope<'a, T>) -> Element) -> T::Builder {
    T::builder()
}
