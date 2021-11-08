//! # VirtualDOM Implementation for Rust
//!
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
//! In this file, multiple items are defined. This file is big, but should be documented well to
//! navigate the inner workings of the Dom. We try to keep these main mechanics in this file to limit
//! the possible exposed API surface (keep fields private). This particular implementation of VDOM
//! is extremely efficient, but relies on some unsafety under the hood to do things like manage
//! micro-heaps for components. We are currently working on refactoring the safety out into safe(r)
//! abstractions, but current tests (MIRI and otherwise) show no issues with the current implementation.
//!
//! Included is:
//! - The [`VirtualDom`] itself
//! - The [`Scope`] object for managing component lifecycle
//! - The [`ActiveFrame`] object for managing the Scope`s microheap
//! - The [`Context`] object for exposing VirtualDOM API to components
//! - The [`NodeFactory`] object for lazily exposing the `Context` API to the nodebuilder API
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.

use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Future, StreamExt};
use fxhash::FxHashSet;
use indexmap::IndexSet;
use std::pin::Pin;
use std::task::Poll;
use std::{any::Any, collections::VecDeque};

/// An integrated virtual node system that progresses events and diffs UI trees.
///
/// Differences are converted into patches which a renderer can use to draw the UI.
///
/// If you are building an App with Dioxus, you probably won't want to reach for this directly, instead opting to defer
/// to a particular crate's wrapper over the [`VirtualDom`] API.
///
/// Example
/// ```rust
/// static App: FC<()> = |(cx, props)|{
///     cx.render(rsx!{
///         div {
///             "Hello World"
///         }
///     })
/// }
///
/// async fn main() {
///     let mut dom = VirtualDom::new(App);
///     let mut inital_edits = dom.rebuild();
///     initialize_screen(inital_edits);
///
///     loop {
///         let next_frame = TimeoutFuture::new(Duration::from_millis(16));
///         let edits = dom.run_with_deadline(next_frame).await;
///         apply_edits(edits);
///         render_frame();
///     }
/// }
/// ```
pub struct VirtualDom {
    base_scope: ScopeId,

    _root_caller: *mut dyn Fn(&Scope) -> Element,

    pub(crate) scopes: ScopeArena,

    receiver: UnboundedReceiver<SchedulerMsg>,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,

    // Every component that has futures that need to be polled
    pending_futures: FxHashSet<ScopeId>,
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
    /// ```
    /// fn Example(cx: Context<()>) -> DomTree  {
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
    /// ```
    /// #[derive(PartialEq, Props)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: Context<SomeProps>) -> DomTree  {
    ///     cx.render(rsx!{ div{ "hello {cx.name}" } })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDOM is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// let mutations = dom.rebuild();
    /// ```
    pub fn new_with_props<P: 'static + Send>(root: FC<P>, root_props: P) -> Self {
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
        let mut scopes = ScopeArena::new(sender.clone());

        let caller = Box::new(move |f: &Scope| -> Element { root(f, &root_props) });
        let caller_ref: *mut dyn Fn(&Scope) -> Element = Box::into_raw(caller);
        let base_scope = scopes.new_with_key(root as _, caller_ref, None, 0, 0);

        Self {
            scopes,
            base_scope,
            receiver,
            // todo: clean this up manually?
            _root_caller: caller_ref,
            pending_messages: VecDeque::new(),
            pending_futures: Default::default(),
            dirty_scopes: Default::default(),
            sender,
        }
    }

    /// Get the [`ScopeState`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or alternsative renderers that use Dioxus
    /// directly.
    ///
    /// # Example
    pub fn base_scope(&self) -> &Scope {
        self.get_scope(&self.base_scope).unwrap()
    }

    /// Get the [`ScopeState`] for a component given its [`ScopeId`]
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
    ///
    ///    
    pub fn get_scheduler_channel(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        self.sender.clone()
    }

    /// Check if the [`VirtualDom`] has any pending updates or work to be done.
    ///
    /// # Example
    ///
    ///
    ///
    pub fn has_any_work(&self) -> bool {
        !(self.dirty_scopes.is_empty() && self.pending_messages.is_empty())
    }

    /// Waits for the scheduler to have work
    /// This lets us poll async tasks during idle periods without blocking the main thread.
    pub async fn wait_for_work(&mut self) {
        // todo: poll the events once even if there is work to do to prevent starvation

        // if there's no futures in the virtualdom, just wait for a scheduler message and put it into the queue to be processed
        if self.pending_futures.is_empty() {
            self.pending_messages
                .push_front(self.receiver.next().await.unwrap());
        } else {
            struct PollTasks<'a> {
                pending_futures: &'a FxHashSet<ScopeId>,
                scopes: &'a ScopeArena,
            }

            impl<'a> Future for PollTasks<'a> {
                type Output = ();

                fn poll(
                    self: Pin<&mut Self>,
                    cx: &mut std::task::Context<'_>,
                ) -> Poll<Self::Output> {
                    let mut all_pending = true;

                    // Poll every scope manually
                    for fut in self.pending_futures.iter() {
                        let scope = self
                            .scopes
                            .get_scope(fut)
                            .expect("Scope should never be moved");

                        let mut items = scope.items.borrow_mut();
                        for task in items.tasks.iter_mut() {
                            let task = task.as_mut();

                            // todo: does this make sense?
                            // I don't usually write futures by hand
                            // I think the futures neeed to be pinned using bumpbox or something
                            // right now, they're bump allocated so this shouldn't matter anyway - they're not going to move
                            let unpinned = unsafe { Pin::new_unchecked(task) };

                            if unpinned.poll(cx).is_ready() {
                                all_pending = false
                            }
                        }
                    }

                    // Resolve the future if any singular task is ready
                    match all_pending {
                        true => Poll::Pending,
                        false => Poll::Ready(()),
                    }
                }
            }

            // Poll both the futures and the scheduler message queue simulataneously
            use futures_util::future::{select, Either};

            let scheduler_fut = self.receiver.next();
            let tasks_fut = PollTasks {
                pending_futures: &self.pending_futures,
                scopes: &self.scopes,
            };

            match select(tasks_fut, scheduler_fut).await {
                // Futures don't generate work
                Either::Left((_, _)) => {}

                // Save these messages in FIFO to be processed later
                Either::Right((msg, _)) => self.pending_messages.push_front(msg.unwrap()),
            }
        }
    }

    /// Run the virtualdom with a deadline.
    ///
    /// This method will progress async tasks until the deadline is reached. If tasks are completed before the deadline,
    /// and no tasks are pending, this method will return immediately. If tasks are still pending, then this method will
    /// exhaust the deadline working on them.
    ///
    /// This method is useful when needing to schedule the virtualdom around other tasks on the main thread to prevent
    /// "jank". It will try to finish whatever work it has by the deadline to free up time for other work.
    ///
    /// Due to platform differences in how time is handled, this method accepts a future that resolves when the deadline
    /// is exceeded. However, the deadline won't be met precisely, so you might want to build some wiggle room into the
    /// deadline closure manually.
    ///
    /// The deadline is polled before starting to diff components. This strikes a balance between the overhead of checking
    /// the deadline and just completing the work. However, if an individual component takes more than 16ms to render, then
    /// the screen will "jank" up. In debug, this will trigger an alert.
    ///
    /// If there are no in-flight fibers when this method is called, it will await any possible tasks, aborting early if
    /// the provided deadline future resolves.
    ///
    /// For use in the web, it is expected that this method will be called to be executed during "idle times" and the
    /// mutations to be applied during the "paint times" IE "animation frames". With this strategy, it is possible to craft
    /// entirely jank-free applications that perform a ton of work.
    ///
    /// # Example
    ///
    /// ```no_run
    /// static App: FC<()> = |(cx, props)|rsx!(cx, div {"hello"} );
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
    ///
    /// ## Mutations
    ///
    /// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
    /// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
    /// applied the edits.
    ///
    /// Mutations are the only link between the RealDOM and the VirtualDOM.
    pub fn work_with_deadline(&mut self, mut deadline: impl FnMut() -> bool) -> Vec<Mutations> {
        let mut committed_mutations = vec![];

        while self.has_any_work() {
            while let Ok(Some(msg)) = self.receiver.try_next() {
                self.pending_messages.push_front(msg);
            }

            while let Some(msg) = self.pending_messages.pop_back() {
                match msg {
                    SchedulerMsg::Immediate(id) => {
                        self.dirty_scopes.insert(id);
                    }
                    SchedulerMsg::UiEvent(event) => {
                        if let Some(element) = event.mounted_dom_id {
                            log::info!("Calling listener {:?}, {:?}", event.scope_id, element);

                            let scope = self.scopes.get_scope(&event.scope_id).unwrap();

                            // TODO: bubble properly here
                            scope.call_listener(event, element);

                            while let Ok(Some(dirty_scope)) = self.receiver.try_next() {
                                self.pending_messages.push_front(dirty_scope);
                            }
                        } else {
                            log::debug!("User event without a targetted ElementId. Not currently supported.\nUnsure how to proceed. {:?}", event);
                        }
                    }
                }
            }

            let mut diff_state: DiffState = DiffState::new(Mutations::new());

            let mut ran_scopes = FxHashSet::default();

            // todo: the 2021 version of rust will let us not have to force the borrow
            let scopes = &self.scopes;

            // Sort the scopes by height. Theoretically, we'll de-duplicate scopes by height
            self.dirty_scopes
                .retain(|id| scopes.get_scope(id).is_some());

            self.dirty_scopes.sort_by(|a, b| {
                let h1 = scopes.get_scope(a).unwrap().height;
                let h2 = scopes.get_scope(b).unwrap().height;
                h1.cmp(&h2).reverse()
            });

            if let Some(scopeid) = self.dirty_scopes.pop() {
                log::info!("handling dirty scope {:?}", scopeid);

                if !ran_scopes.contains(&scopeid) {
                    ran_scopes.insert(scopeid);

                    log::debug!("about to run scope {:?}", scopeid);

                    if self.scopes.run_scope(&scopeid) {
                        let scope = self.scopes.get_scope(&scopeid).unwrap();
                        let (old, new) = (scope.wip_head(), scope.fin_head());
                        diff_state.stack.scope_stack.push(scopeid);
                        diff_state.stack.push(DiffInstruction::Diff { new, old });
                    }
                }
            }

            let work_completed = self.scopes.work(&mut diff_state, &mut deadline);

            if work_completed {
                let DiffState {
                    mutations,
                    seen_scopes,
                    stack,
                    ..
                } = diff_state;

                for scope in seen_scopes {
                    self.dirty_scopes.remove(&scope);
                }

                // I think the stack should be empty at the end of diffing?
                debug_assert_eq!(stack.scope_stack.len(), 0);

                committed_mutations.push(mutations);
            } else {
                // leave the work in an incomplete state
                log::debug!("don't have a mechanism to pause work (yet)");
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
    /// ```
    /// static App: FC<()> = |(cx, props)| cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        // todo: I think we need to append a node or something
        //     diff_machine
        //         .stack
        //         .create_node(cur_component.frames.fin_head(), MountType::Append);

        let scope = self.base_scope;
        self.hard_diff(&scope).unwrap()
    }

    /// Compute a manual diff of the VirtualDOM between states.
    ///
    /// This can be useful when state inside the DOM is remotely changed from the outside, but not propagated as an event.
    ///
    /// In this case, every component will be diffed, even if their props are memoized. This method is intended to be used
    /// to force an update of the DOM when the state of the app is changed outside of the app.
    ///
    ///
    /// # Example
    /// ```rust
    /// #[derive(PartialEq, Props)]
    /// struct AppProps {
    ///     value: Shared<&'static str>,
    /// }
    ///
    /// static App: FC<AppProps> = |(cx, props)|{
    ///     let val = cx.value.borrow();
    ///     cx.render(rsx! { div { "{val}" } })
    /// };
    ///
    /// let value = Rc::new(RefCell::new("Hello"));
    /// let mut dom = VirtualDom::new_with_props(
    ///     App,
    ///     AppProps {
    ///         value: value.clone(),
    ///     },
    /// );
    ///
    /// let _ = dom.rebuild();
    ///
    /// *value.borrow_mut() = "goodbye";
    ///
    /// let edits = dom.diff();
    /// ```
    pub fn hard_diff<'a>(&'a mut self, scope_id: &ScopeId) -> Option<Mutations<'a>> {
        log::debug!("hard diff {:?}", scope_id);

        if self.scopes.run_scope(scope_id) {
            let mut diff_machine = DiffState::new(Mutations::new());

            diff_machine.force_diff = true;

            self.scopes.diff_scope(&mut diff_machine, scope_id);

            Some(diff_machine.mutations)
        } else {
            None
        }
    }
}

pub enum SchedulerMsg {
    // events from the host
    UiEvent(UserEvent),

    // setstate
    Immediate(ScopeId),
}

#[derive(Debug)]
pub struct UserEvent {
    /// The originator of the event trigger
    pub scope_id: ScopeId,

    pub priority: EventPriority,

    /// The optional real node associated with the trigger
    pub mounted_dom_id: Option<ElementId>,

    /// The event type IE "onclick" or "onmouseover"
    ///
    /// The name that the renderer will use to mount the listener.
    pub name: &'static str,

    /// Event Data
    pub event: Box<dyn Any + Send>,
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
