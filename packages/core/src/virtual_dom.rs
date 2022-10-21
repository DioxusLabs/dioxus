//! # Virtual DOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use std::{collections::VecDeque, iter::FromIterator, task::Poll};

use crate::diff::DiffState;
use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future::poll_fn, StreamExt};
use rustc_hash::FxHashSet;

/// A virtual node system that progresses user events and diffs UI trees.
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
/// fn App(cx: Scope) -> Element {
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
    root: ElementId,
    scopes: ScopeArena,

    pending_messages: VecDeque<SchedulerMsg>,
    dirty_scopes: Vec<ScopeId>,
    removed_scopes: FxHashSet<ScopeId>,

    channel: (
        UnboundedSender<SchedulerMsg>,
        UnboundedReceiver<SchedulerMsg>,
    ),
}

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub enum SchedulerMsg {
    /// Events from the Renderer
    Event(UserEvent),

    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// Mark all components as dirty and update them
    DirtyAll,

    #[cfg(any(feature = "hot-reload", debug_assertions))]
    /// Mark a template as dirty, used for hot reloading
    SetTemplate(Box<SetTemplateMsg>),

    /// New tasks from components that should be polled when the next poll is ready
    NewTask(ScopeId),
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
    /// fn Example(cx: Scope) -> Element  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(root: Component) -> Self {
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
    /// ```rust, ignore
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
            root as ComponentPtr,
            Box::new(VComponentProps {
                props: root_props,
                memo: |_a, _b| unreachable!("memo on root will neve be run"),
                render_fn: root,
            }),
            None,
            ElementId(0),
        );

        Self {
            root: ElementId(0),
            scopes,
            channel,
            dirty_scopes: Vec::from_iter([ScopeId(0)]),
            pending_messages: VecDeque::new(),
            removed_scopes: FxHashSet::default(),
        }
    }

    /// Get the [`Scope`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or alternative renderers that use Dioxus
    /// directly.
    ///
    /// This method is equivalent to calling `get_scope(ScopeId(0))`
    ///
    /// # Example
    ///
    /// ```rust, ignore
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

    /// Try to get an element from its ElementId
    pub fn get_element(&self, id: ElementId) -> Option<&VNode> {
        self.scopes.get_element(id)
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

                    let scopes = &mut self.scopes;
                    let task_poll = poll_fn(|cx| {
                        let mut tasks = scopes.tasks.tasks.borrow_mut();
                        tasks.retain(|_, task| task.as_mut().poll(cx).is_pending());

                        match tasks.is_empty() {
                            true => Poll::Ready(()),
                            false => Poll::Pending,
                        }
                    });

                    match select(task_poll, self.channel.1.next()).await {
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

    /// Handle an individual message for the scheduler.
    ///
    /// This will either call an event listener or mark a component as dirty.
    pub fn process_message(&mut self, msg: SchedulerMsg) {
        match msg {
            SchedulerMsg::NewTask(_id) => {
                // uh, not sure? I think end up re-polling it anyways
            }
            SchedulerMsg::Event(event) => {
                if let Some(element) = event.element {
                    self.scopes.call_listener_with_bubbling(&event, element);
                }
            }
            SchedulerMsg::Immediate(s) => {
                self.mark_dirty_scope(s);
            }
            SchedulerMsg::DirtyAll => {
                let dirty = self
                    .scopes
                    .scopes
                    .borrow()
                    .keys()
                    .copied()
                    .collect::<Vec<_>>();
                for id in dirty {
                    self.mark_dirty_scope(id);
                }
            }
            #[cfg(any(feature = "hot-reload", debug_assertions))]
            SchedulerMsg::SetTemplate(msg) => {
                let SetTemplateMsg(id, tmpl) = *msg;
                if self
                    .scopes
                    .templates
                    .borrow_mut()
                    .insert(
                        id.clone(),
                        std::rc::Rc::new(std::cell::RefCell::new(Template::Owned(tmpl))),
                    )
                    .is_some()
                {
                    self.scopes.template_resolver.borrow_mut().mark_dirty(&id)
                }

                // mark any scopes that used the template as dirty
                self.process_message(SchedulerMsg::DirtyAll);
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
    /// fn App(cx: Scope) -> Element {
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
    #[allow(unused)]
    pub fn work_with_deadline(&mut self, mut deadline: impl FnMut() -> bool) -> Vec<Mutations> {
        let mut committed_mutations = vec![];
        self.scopes.template_bump.reset();
        self.removed_scopes.clear();

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
                if scopes.get_scope(scopeid).is_some()
                    && !self.removed_scopes.contains(&scopeid)
                    && !ran_scopes.contains(&scopeid)
                {
                    ran_scopes.insert(scopeid);

                    self.scopes.run_scope(scopeid);

                    diff_state.diff_scope(self.root, scopeid);

                    let DiffState { mutations, .. } = diff_state;

                    self.removed_scopes
                        .extend(mutations.dirty_scopes.iter().copied());

                    if !mutations.edits.is_empty() {
                        committed_mutations.push(mutations);
                    }

                    // todo: pause the diff machine
                    // if diff_state.work(&mut deadline) {
                    //     let DiffState { mutations, .. } = diff_state;
                    //     for scope in &mutations.dirty_scopes {
                    //         self.dirty_scopes.remove(scope);
                    //     }
                    //     committed_mutations.push(mutations);
                    // } else {
                    //     // leave the work in an incomplete state
                    //     //
                    //     // todo: we should store the edits and re-apply them later
                    //     // for now, we just dump the work completely (threadsafe)
                    //     return committed_mutations;
                    // }
                }
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
    /// static App: Component = |cx|  cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        let scope_id = ScopeId(0);

        let mut diff_state = DiffState::new(&self.scopes);
        self.scopes.run_scope(scope_id);

        diff_state.scope_stack.push(scope_id);

        let node = self.scopes.fin_head(scope_id);
        let mut created = Vec::new();
        diff_state.create_node(self.root, node, &mut created);

        diff_state
            .mutations
            .append_children(Some(self.root.as_u64()), created);

        self.dirty_scopes.clear();
        assert!(self.dirty_scopes.is_empty());

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
    /// static App: Component<AppProps> = |cx| {
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
    /// let edits = dom.hard_diff(ScopeId(0));
    /// ```
    pub fn hard_diff(&mut self, scope_id: ScopeId) -> Mutations {
        let mut diff_machine = DiffState::new(&self.scopes);
        self.scopes.run_scope(scope_id);

        let (old, new) = (
            diff_machine.scopes.wip_head(scope_id),
            diff_machine.scopes.fin_head(scope_id),
        );

        diff_machine.force_diff = true;
        diff_machine.scope_stack.push(scope_id);
        let scope = diff_machine.scopes.get_scope(scope_id).unwrap();

        diff_machine.diff_node(scope.container, old, new);

        diff_machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    /// ```rust, ignore
    /// fn Base(cx: Scope) -> Element {
    ///     render!(div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn render_vnodes<'a>(&'a self, lazy_nodes: LazyNodes<'a, '_>) -> &'a VNode<'a> {
        let scope = self.scopes.get_scope(ScopeId(0)).unwrap();
        let frame = scope.wip_frame();
        let factory = NodeFactory::new(scope);
        let node = lazy_nodes.call(factory);
        frame.bump.alloc(node)
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    /// ```rust, ignore
    /// fn Base(cx: Scope) -> Element {
    ///     render!(div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn diff_vnodes<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Mutations<'a> {
        let mut machine = DiffState::new(&self.scopes);
        machine.scope_stack.push(ScopeId(0));
        machine.diff_node(self.root, old, new);

        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    ///
    ///
    /// ```rust, ignore
    /// fn Base(cx: Scope) -> Element {
    ///     render!(div {})
    /// }
    ///
    /// let dom = VirtualDom::new(Base);
    /// let nodes = dom.render_nodes(rsx!("div"));
    /// ```
    pub fn create_vnodes<'a>(&'a self, nodes: LazyNodes<'a, '_>) -> Mutations<'a> {
        let mut machine = DiffState::new(&self.scopes);
        machine.scope_stack.push(ScopeId(0));
        let node = self.render_vnodes(nodes);
        let mut created = Vec::new();
        machine.create_node(self.root, node, &mut created);
        machine
            .mutations
            .append_children(Some(self.root.as_u64()), created);
        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scopes's arena.
    ///
    /// Useful when needing to diff two rsx! calls from outside the VirtualDom, such as in a test.
    ///
    ///
    /// ```rust, ignore
    /// fn Base(cx: Scope) -> Element {
    ///     render!(div {})
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
        create.scope_stack.push(ScopeId(0));
        let mut created = Vec::new();
        create.create_node(self.root, old, &mut created);
        create
            .mutations
            .append_children(Some(self.root.as_u64()), created);

        let mut edit = DiffState::new(&self.scopes);
        edit.scope_stack.push(ScopeId(0));
        edit.diff_node(self.root, old, new);

        (create.mutations, edit.mutations)
    }

    /// Runs a function with the template associated with a given id.
    pub fn with_template<R>(&self, id: &TemplateId, mut f: impl FnMut(&Template) -> R) -> R {
        self.scopes
            .templates
            .borrow()
            .get(id)
            .map(|inner| {
                let borrow = inner;
                f(&borrow.borrow())
            })
            .unwrap()
    }

    fn mark_dirty_scope(&mut self, scope_id: ScopeId) {
        let scopes = &self.scopes;
        if let Some(scope) = scopes.get_scope(scope_id) {
            let height = scope.height;
            let id = scope_id.0;
            if let Err(index) = self.dirty_scopes.binary_search_by(|new| {
                let scope = scopes.get_scope(*new).unwrap();
                let new_height = scope.height;
                let new_id = &scope.scope_id();
                height.cmp(&new_height).then(new_id.0.cmp(&id))
            }) {
                self.dirty_scopes.insert(index, scope_id);
                log::info!("mark_dirty_scope: {:?}", self.dirty_scopes);
            }
        }
    }
}

/*
Scopes and ScopeArenas are never dropped internally.
An app will always occupy as much memory as its biggest form.

This means we need to handle all specifics of drop *here*. It's easier
to reason about centralizing all the drop
logic in one spot rather than scattered in each module.

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
