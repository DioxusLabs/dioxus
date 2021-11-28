//! # VirtualDOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.

use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Future, StreamExt};
use fxhash::FxHashSet;
use indexmap::IndexSet;
use smallvec::SmallVec;
use std::{any::Any, collections::VecDeque, pin::Pin, sync::Arc, task::Poll};

/// A virtual node system that progresses user events and diffs UI trees.
///
///
/// ## Guide
///
/// Components are defined as simple functions that take [`Context`] and a [`Properties`] type and return an [`Element`].  
///
/// ```rust, ignore
/// #[derive(Props, PartialEq)]
/// struct AppProps {
///     title: String
/// }
///
/// fn App(cx: Context, props: &AppProps) -> Element {
///     cx.render(rsx!(
///         div {"hello, {props.title}"}
///     ))
/// }
/// ```
///
/// Components may be composed to make complex apps.
///
/// ```rust, ignore
/// fn App(cx: Context, props: &AppProps) -> Element {
///     cx.render(rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{props.title}" }
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
/// fn App(cx: Context, props: &()) -> Element {
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
    base_scope: ScopeId,

    _root_props: Box<dyn Any>,

    scopes: Box<ScopeArena>,

    receiver: UnboundedReceiver<SchedulerMsg>,

    sender: UnboundedSender<SchedulerMsg>,

    pending_messages: VecDeque<SchedulerMsg>,

    dirty_scopes: IndexSet<ScopeId>,
}

// Methods to create the VirtualDom
impl VirtualDom {
    /// Create a new VirtualDOM with a component that does not have special props.
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
    /// fn Example(cx: Context, props: &()) -> Element  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Create a new VirtualDOM with the given properties for the root component.
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
    /// fn Example(cx: Context, props: &SomeProps) -> Element  {
    ///     cx.render(rsx!{ div{ "hello {cx.name}" } })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// let mutations = dom.rebuild();
    /// ```
    pub fn new_with_props<P: 'static>(root: FC<P>, root_props: P) -> Self {
        let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();
        Self::new_with_props_and_scheduler(root, root_props, sender, receiver)
    }

    /// Launch the VirtualDom, but provide your own channel for receiving and sending messages into the scheduler
    ///
    /// This is useful when the VirtualDom must be driven from outside a thread and it doesn't make sense to wait for the
    /// VirtualDom to be created just to retrieve its channel receiver.
    pub fn new_with_props_and_scheduler<P: 'static>(
        root: FC<P>,
        root_props: P,
        sender: UnboundedSender<SchedulerMsg>,
        receiver: UnboundedReceiver<SchedulerMsg>,
    ) -> Self {
        let scopes = ScopeArena::new(sender.clone());

        let mut caller = Box::new(move |scp: &Scope| -> Element { root(scp, &root_props) });
        let caller_ref: *mut dyn Fn(&Scope) -> Element = caller.as_mut() as *mut _;
        let base_scope = scopes.new_with_key(root as _, caller_ref, None, ElementId(0), 0, 0);

        let pending_messages = VecDeque::new();
        let mut dirty_scopes = IndexSet::new();
        dirty_scopes.insert(base_scope);

        Self {
            scopes: Box::new(scopes),
            base_scope,
            receiver,
            _root_props: caller,
            pending_messages,
            dirty_scopes,
            sender,
        }
    }

    /// Get the [`Scope`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or alternsative renderers that use Dioxus
    /// directly.
    ///
    /// # Example
    pub fn base_scope(&self) -> &Scope {
        self.get_scope(&self.base_scope).unwrap()
    }

    /// Get the [`Scope`] for a component given its [`ScopeId`]
    ///
    /// # Example
    ///
    ///
    ///
    pub fn get_scope<'a>(&'a self, id: &ScopeId) -> Option<&'a Scope> {
        self.scopes.get_scope(id)
    }

    /// Get an [`UnboundedSender`] handle to the channel used by the scheduler.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    ///
    ///
    /// ```
    pub fn get_scheduler_channel(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        self.sender.clone()
    }

    /// Check if the [`VirtualDom`] has any pending updates or work to be done.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    ///
    ///
    /// ```
    pub fn has_any_work(&self) -> bool {
        !(self.dirty_scopes.is_empty() && self.pending_messages.is_empty())
    }

    /// Waits for the scheduler to have work
    /// This lets us poll async tasks during idle periods without blocking the main thread.
    pub async fn wait_for_work(&mut self) {
        loop {
            if !self.dirty_scopes.is_empty() && self.pending_messages.is_empty() {
                break;
            }

            if self.pending_messages.is_empty() {
                if self.scopes.pending_futures.borrow().is_empty() {
                    self.pending_messages
                        .push_front(self.receiver.next().await.unwrap());
                } else {
                    use futures_util::future::{select, Either};

                    match select(PollTasks(&mut self.scopes), self.receiver.next()).await {
                        Either::Left((_, _)) => {}
                        Either::Right((msg, _)) => self.pending_messages.push_front(msg.unwrap()),
                    }
                }
            }

            while let Ok(Some(msg)) = self.receiver.try_next() {
                self.pending_messages.push_front(msg);
            }

            if let Some(msg) = self.pending_messages.pop_back() {
                match msg {
                    // just keep looping, the task is now saved but we should actually poll it
                    SchedulerMsg::NewTask(id) => {
                        self.scopes.pending_futures.borrow_mut().insert(id);
                    }
                    SchedulerMsg::UiEvent(event) => {
                        if let Some(element) = event.element {
                            self.scopes.call_listener_with_bubbling(event, element);
                        }
                    }
                    SchedulerMsg::Immediate(s) => {
                        self.dirty_scopes.insert(s);
                    }
                }
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
    /// fn App(cx: Context, props: &()) -> Element {
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
            // log::debug!("working with deadline");
            let scopes = &self.scopes;
            let mut diff_state = DiffState::new(scopes);

            let mut ran_scopes = FxHashSet::default();

            // Sort the scopes by height. Theoretically, we'll de-duplicate scopes by height
            self.dirty_scopes
                .retain(|id| scopes.get_scope(id).is_some());
            self.dirty_scopes.sort_by(|a, b| {
                let h1 = scopes.get_scope(a).unwrap().height;
                let h2 = scopes.get_scope(b).unwrap().height;
                h1.cmp(&h2).reverse()
            });

            if let Some(scopeid) = self.dirty_scopes.pop() {
                if !ran_scopes.contains(&scopeid) {
                    ran_scopes.insert(scopeid);

                    if self.scopes.run_scope(&scopeid) {
                        let (old, new) = (
                            self.scopes.wip_head(&scopeid),
                            self.scopes.fin_head(&scopeid),
                        );
                        diff_state.stack.push(DiffInstruction::Diff { new, old });
                        diff_state.stack.scope_stack.push(scopeid);

                        let scope = scopes.get_scope(&scopeid).unwrap();
                        diff_state.stack.element_stack.push(scope.container);
                    }
                }
            }

            if diff_state.work(&mut deadline) {
                let DiffState {
                    mutations,
                    seen_scopes,
                    ..
                } = diff_state;

                for scope in seen_scopes {
                    self.dirty_scopes.remove(&scope);
                }

                committed_mutations.push(mutations);
            } else {
                // leave the work in an incomplete state
                return committed_mutations;
            }
        }

        committed_mutations
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom from scratch
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
    /// static App: FC<()> = |cx, props| cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        let mut diff_state = DiffState::new(&self.scopes);

        let scope_id = self.base_scope;
        if self.scopes.run_scope(&scope_id) {
            diff_state
                .stack
                .create_node(self.scopes.fin_head(&scope_id), MountType::Append);

            diff_state.stack.element_stack.push(ElementId(0));
            diff_state.stack.scope_stack.push(scope_id);

            diff_state.work(|| false);
        }

        diff_state.mutations
    }

    /// Compute a manual diff of the VirtualDOM between states.
    ///
    /// This can be useful when state inside the DOM is remotely changed from the outside, but not propagated as an event.
    ///
    /// In this case, every component will be diffed, even if their props are memoized. This method is intended to be used
    /// to force an update of the DOM when the state of the app is changed outside of the app.
    ///
    /// # Example
    /// ```rust, ignore
    /// #[derive(PartialEq, Props)]
    /// struct AppProps {
    ///     value: Shared<&'static str>,
    /// }
    ///
    /// static App: FC<AppProps> = |cx, props|{
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
    pub fn hard_diff<'a>(&'a mut self, scope_id: &ScopeId) -> Option<Mutations<'a>> {
        let mut diff_machine = DiffState::new(&self.scopes);
        if self.scopes.run_scope(scope_id) {
            diff_machine.force_diff = true;
            diff_machine.diff_scope(scope_id);
        }
        Some(diff_machine.mutations)
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    pub fn render_vnodes<'a>(&'a self, lazy_nodes: Option<LazyNodes<'a, '_>>) -> &'a VNode<'a> {
        let scope = self.scopes.get_scope(&self.base_scope).unwrap();
        let frame = scope.wip_frame();
        let factory = NodeFactory { bump: &frame.bump };
        let node = lazy_nodes.unwrap().call(factory);
        frame.bump.alloc(node)
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.    
    pub fn diff_vnodes<'a>(&'a self, old: &'a VNode<'a>, new: &'a VNode<'a>) -> Mutations<'a> {
        let mut machine = DiffState::new(&self.scopes);
        machine.stack.push(DiffInstruction::Diff { new, old });
        machine.stack.element_stack.push(ElementId(0));
        machine.stack.scope_stack.push(self.base_scope);
        machine.work(|| false);
        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    pub fn create_vnodes<'a>(&'a self, left: Option<LazyNodes<'a, '_>>) -> Mutations<'a> {
        let nodes = self.render_vnodes(left);
        let mut machine = DiffState::new(&self.scopes);
        machine.stack.element_stack.push(ElementId(0));
        machine.stack.create_node(nodes, MountType::Append);
        machine.work(|| false);
        machine.mutations
    }

    /// Renders an `rsx` call into the Base Scope's allocator.
    ///
    /// Useful when needing to render nodes from outside the VirtualDom, such as in a test.
    pub fn diff_lazynodes<'a>(
        &'a self,
        left: Option<LazyNodes<'a, '_>>,
        right: Option<LazyNodes<'a, '_>>,
    ) -> (Mutations<'a>, Mutations<'a>) {
        let (old, new) = (self.render_vnodes(left), self.render_vnodes(right));

        let mut create = DiffState::new(&self.scopes);
        create.stack.scope_stack.push(self.base_scope);
        create.stack.element_stack.push(ElementId(0));
        create.stack.create_node(old, MountType::Append);
        create.work(|| false);

        let mut edit = DiffState::new(&self.scopes);
        edit.stack.scope_stack.push(self.base_scope);
        edit.stack.element_stack.push(ElementId(0));
        edit.stack.push(DiffInstruction::Diff { old, new });
        edit.work(&mut || false);

        (create.mutations, edit.mutations)
    }
}

pub enum SchedulerMsg {
    // events from the host
    UiEvent(UserEvent),

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
/// fn App(cx: Context, props: &()) -> Element {
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
    /// The originator of the event trigger
    pub scope_id: Option<ScopeId>,

    pub priority: EventPriority,

    /// The optional real node associated with the trigger
    pub element: Option<ElementId>,

    /// The event type IE "onclick" or "onmouseover"
    ///
    /// The name that the renderer will use to mount the listener.
    pub name: &'static str,

    /// Event Data
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

struct PollTasks<'a>(&'a mut ScopeArena);

impl<'a> Future for PollTasks<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut all_pending = true;

        let mut unfinished_tasks: SmallVec<[_; 10]> = smallvec::smallvec![];
        let mut scopes_to_clear: SmallVec<[_; 10]> = smallvec::smallvec![];

        // Poll every scope manually
        for fut in self.0.pending_futures.borrow().iter() {
            let scope = self.0.get_scope(fut).expect("Scope should never be moved");

            let mut items = scope.items.borrow_mut();

            // really this should just be retain_mut but that doesn't exist yet
            while let Some(mut task) = items.tasks.pop() {
                // todo: does this make sense?
                // I don't usually write futures by hand
                // I think the futures neeed to be pinned using bumpbox or something
                // right now, they're bump allocated so this shouldn't matter anyway - they're not going to move
                let task_mut = task.as_mut();
                let pinned = unsafe { Pin::new_unchecked(task_mut) };

                if pinned.poll(cx).is_ready() {
                    all_pending = false
                } else {
                    unfinished_tasks.push(task);
                }
            }

            if unfinished_tasks.is_empty() {
                scopes_to_clear.push(*fut);
            }

            items.tasks.extend(unfinished_tasks.drain(..));
        }

        for scope in scopes_to_clear {
            self.0.pending_futures.borrow_mut().remove(&scope);
        }

        // Resolve the future if any singular task is ready
        match all_pending {
            true => Poll::Pending,
            false => Poll::Ready(()),
        }
    }
}
