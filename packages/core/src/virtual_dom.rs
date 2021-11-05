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
use bumpalo::Bump;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{pin_mut, stream::FuturesUnordered, Future, FutureExt, StreamExt};
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use indexmap::IndexSet;
use slab::Slab;
use std::pin::Pin;
use std::task::Poll;
use std::{
    any::{Any, TypeId},
    cell::{Cell, UnsafeCell},
    collections::{HashSet, VecDeque},
    rc::Rc,
};

use crate::innerlude::*;

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

    root_fc: Box<dyn Any>,

    root_props: Rc<dyn Any>,

    // we need to keep the allocation around, but we don't necessarily use it
    _root_caller: Box<dyn Any>,

    // /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    // /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    // ///
    // /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    // /// and rusts's guarantees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    // pub pool: ResourcePool,
    //
    pub component_arena: Bump,

    pub free_components: VecDeque<*mut ScopeInner>,

    pub heuristics: FxHashMap<FcSlot, Heuristic>,

    pub receiver: UnboundedReceiver<SchedulerMsg>,

    // Garbage stored
    pub pending_garbage: FxHashSet<ScopeId>,

    // Every component that has futures that need to be polled
    pub pending_futures: FxHashSet<ScopeId>,

    pub ui_events: VecDeque<UserEvent>,

    pub pending_immediates: VecDeque<ScopeId>,

    pub batched_events: VecDeque<UserEvent>,

    pub garbage_scopes: HashSet<ScopeId>,

    pub dirty_scopes: IndexSet<ScopeId>,

    pub saved_state: Option<SavedDiffWork<'static>>,

    pub in_progress: bool,
}

pub enum SchedulerMsg {
    // events from the host
    UiEvent(UserEvent),

    // setstate
    Immediate(ScopeId),
}

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

    /// Launch the VirtualDom, but provide your own channel for receiving and sending messages into the
    ///
    /// This is useful when the VirtualDom must be driven from outside a thread and it doesn't make sense to wait for the
    /// VirtualDom to be created just to retrieve its channel receiver.
    pub fn new_with_props_and_scheduler<P: 'static>(
        root: FC<P>,
        root_props: P,
        sender: UnboundedSender<SchedulerMsg>,
        receiver: UnboundedReceiver<SchedulerMsg>,
    ) -> Self {
        let mut component_arena = Bump::new();

        // let root_fc = Box::new(root);

        // let root_props: Rc<dyn Any> = Rc::new(root_props);

        // let props = root_props.clone();

        // let mut root_caller: Box<dyn Fn(&ScopeInner) -> Element> =
        //     Box::new(move |scope: &ScopeInner| {
        //         let props = props.downcast_ref::<P>().unwrap();
        //         let node = root((scope, props));
        //         // cast into the right lifetime
        //         unsafe { std::mem::transmute(node) }
        //     });

        // let caller = unsafe { bumpalo::boxed::Box::from_raw(root_caller.as_mut() as *mut _) };

        // // todo make the memory footprint congifurable
        // let scheduler = Scheduler::new(sender, receiver, 100, 2000);

        // let vcomp = VComponent {
        //     key: todo!(),
        //     associated_scope: todo!(),
        //     user_fc: root as *const _,
        //     can_memoize: todo!(),
        //     raw_props: todo!(),
        //     // drop_props: todo!(),
        //     // caller,
        //     comparator: todo!(),
        //     caller: todo!(),
        // };

        // let boxed_comp = Box::new(vcomp);

        // let base_scope = pool.insert_scope_with_key(|myidx| {
        //     ScopeInner::new(
        //         boxed_comp.as_ref(),
        //         myidx,
        //         None,
        //         0,
        //         0,
        //         pool.channel.clone(),
        //     )
        // });

        Self {
            scheduler: todo!(),
            base_scope: todo!(),
            root_fc: todo!(),
            root_props: todo!(),
            _root_caller: todo!(),
            component_arena: todo!(),
            free_components: todo!(),
            heuristics: todo!(),
            receiver,
            pending_garbage: todo!(),
            pending_futures: todo!(),
            ui_events: todo!(),
            pending_immediates: todo!(),
            batched_events: todo!(),
            garbage_scopes: todo!(),
            dirty_scopes: todo!(),
            saved_state: todo!(),
            in_progress: todo!(),
        }
    }

    /// Get the [`Scope`] for the root component.
    ///
    /// This is useful for traversing the tree from the root for heuristics or alternsative renderers that use Dioxus
    /// directly.
    pub fn base_scope(&self) -> &ScopeInner {
        self.pool.get_scope(&self.base_scope).unwrap()
    }

    /// Get the [`Scope`] for a component given its [`ScopeId`]
    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeInner> {
        self.pool.get_scope(&id)
    }

    /// Update the root props of this VirtualDOM.
    ///
    /// This method returns None if the old props could not be removed. The entire VirtualDOM will be rebuilt immediately,
    /// so calling this method will block the main thread until computation is done.
    ///
    /// ## Example
    ///
    /// ```rust
    /// #[derive(Props, PartialEq)]
    /// struct AppProps {
    ///     route: &'static str
    /// }
    /// static App: FC<AppProps> = |(cx, props)|cx.render(rsx!{ "route is {cx.route}" });
    ///
    /// let mut dom = VirtualDom::new_with_props(App, AppProps { route: "start" });
    ///
    /// let mutations = dom.update_root_props(AppProps { route: "end" }).unwrap();
    /// ```
    pub fn update_root_props<P>(&mut self, root_props: P) -> Option<Mutations>
    where
        P: 'static,
    {
        let root_scope = self.pool.get_scope_mut(&self.base_scope).unwrap();

        // Pre-emptively drop any downstream references of the old props
        root_scope.ensure_drop_safety(&self.pool);

        let mut root_props: Rc<dyn Any> = Rc::new(root_props);

        if let Some(props_ptr) = root_props.downcast_ref::<P>().map(|p| p as *const P) {
            // Swap the old props and new props
            std::mem::swap(&mut self.root_props, &mut root_props);

            let root = *self.root_fc.downcast_ref::<FC<P>>().unwrap();

            let root_caller: Box<dyn Fn(&ScopeInner) -> Element> =
                Box::new(move |scope: &ScopeInner| unsafe {
                    let props: &'_ P = &*(props_ptr as *const P);
                    std::mem::transmute(root((scope, props)))
                });

            drop(root_props);

            Some(self.rebuild())
        } else {
            None
        }
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
    /// static App: FC<()> = |(cx, props)|cx.render(rsx!{ "hello world" });
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild(&mut self) -> Mutations {
        self.rebuild(self.base_scope)
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
    pub fn diff(&mut self) -> Mutations {
        self.hard_diff(self.base_scope)
    }

    /// Runs the virtualdom immediately, not waiting for any suspended nodes to complete.
    ///
    /// This method will not wait for any suspended nodes to complete. If there is no pending work, then this method will
    /// return "None"
    pub fn run_immediate(&mut self) -> Option<Vec<Mutations>> {
        if self.has_any_work() {
            Some(self.work_sync())
        } else {
            None
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
    /// let mut dom = VirtualDom::new(App);
    /// loop {
    ///     let deadline = TimeoutFuture::from_ms(16);
    ///     let mutations = dom.run_with_deadline(deadline).await;
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
    pub fn run_with_deadline(&mut self, deadline: impl FnMut() -> bool) -> Vec<Mutations<'_>> {
        self.work_with_deadline(deadline)
    }

    pub fn get_event_sender(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        self.pool.channel.sender.clone()
    }

    /// Waits for the scheduler to have work
    /// This lets us poll async tasks during idle periods without blocking the main thread.
    pub async fn wait_for_work(&mut self) {
        // todo: poll the events once even if there is work to do to prevent starvation
        if self.has_any_work() {
            return;
        }

        use futures_util::StreamExt;

        // Wait for any new events if we have nothing to do

        // let tasks_fut = self.async_tasks.next();
        // let scheduler_fut = self.receiver.next();

        // use futures_util::future::{select, Either};
        // match select(tasks_fut, scheduler_fut).await {
        //     // poll the internal futures
        //     Either::Left((_id, _)) => {
        //         //
        //     }

        //     // wait for an external event
        //     Either::Right((msg, _)) => match msg.unwrap() {
        //         SchedulerMsg::Task(t) => {
        //             self.handle_task(t);
        //         }
        //         SchedulerMsg::Immediate(im) => {
        //             self.dirty_scopes.insert(im);
        //         }
        //         SchedulerMsg::UiEvent(evt) => {
        //             self.ui_events.push_back(evt);
        //         }
        //     },
        // }
    }
}

pub type FcSlot = *const ();

pub struct Heuristic {
    hook_arena_size: usize,
    node_arena_size: usize,
}

/*
Welcome to Dioxus's cooperative, priority-based scheduler.

I hope you enjoy your stay.

Some essential reading:
- https://github.com/facebook/react/blob/main/packages/scheduler/src/forks/Scheduler.js#L197-L200
- https://github.com/facebook/react/blob/main/packages/scheduler/src/forks/Scheduler.js#L440
- https://github.com/WICG/is-input-pending
- https://web.dev/rail/
- https://indepth.dev/posts/1008/inside-fiber-in-depth-overview-of-the-new-reconciliation-algorithm-in-react

# What's going on?

Dioxus is a framework for "user experience" - not just "user interfaces." Part of the "experience" is keeping the UI
snappy and "jank free" even under heavy work loads. Dioxus already has the "speed" part figured out - but there's no
point in being "fast" if you can't also be "responsive."

As such, Dioxus can manually decide on what work is most important at any given moment in time. With a properly tuned
priority system, Dioxus can ensure that user interaction is prioritized and committed as soon as possible (sub 100ms).
The controller responsible for this priority management is called the "scheduler" and is responsible for juggling many
different types of work simultaneously.

# How does it work?

Per the RAIL guide, we want to make sure that A) inputs are handled ASAP and B) animations are not blocked.
React-three-fiber is a testament to how amazing this can be - a ThreeJS scene is threaded in between work periods of
React, and the UI still stays snappy!

While it's straightforward to run code ASAP and be as "fast as possible", what's not  _not_ straightforward is how to do
this while not blocking the main thread. The current prevailing thought is to stop working periodically so the browser
has time to paint and run animations. When the browser is finished, we can step in and continue our work.

React-Fiber uses the "Fiber" concept to achieve a pause-resume functionality. This is worth reading up on, but not
necessary to understand what we're doing here. In Dioxus, our DiffMachine is guided by DiffInstructions - essentially
"commands" that guide the Diffing algorithm through the tree. Our "diff_scope" method is async - we can literally pause
our DiffMachine "mid-sentence" (so to speak) by just stopping the poll on the future. The DiffMachine periodically yields
so Rust's async machinery can take over, allowing us to customize when exactly to pause it.

React's "should_yield" method is more complex than ours, and I assume we'll move in that direction as Dioxus matures. For
now, Dioxus just assumes a TimeoutFuture, and selects! on both the Diff algorithm and timeout. If the DiffMachine finishes
before the timeout, then Dioxus will work on any pending work in the interim. If there is no pending work, then the changes
are committed, and coroutines are polled during the idle period. However, if the timeout expires, then the DiffMachine
future is paused and saved (self-referentially).

# Priority System

So far, we've been able to thread our Dioxus work between animation frames - the main thread is not blocked! But that
doesn't help us _under load_. How do we still stay snappy... even if we're doing a lot of work? Well, that's where
priorities come into play. The goal with priorities is to schedule shorter work as a "high" priority and longer work as
a "lower" priority. That way, we can interrupt long-running low-priority work with short-running high-priority work.

React's priority system is quite complex.

There are 5 levels of priority and 2 distinctions between UI events (discrete, continuous). I believe React really only
uses 3 priority levels and "idle" priority isn't used... Regardless, there's some batching going on.

For Dioxus, we're going with a 4 tier priority system:
- Sync: Things that need to be done by the next frame, like TextInput on controlled elements
- High: for events that block all others - clicks, keyboard, and hovers
- Medium: for UI events caused by the user but not directly - scrolls/forms/focus (all other events)
- Low: set_state called asynchronously, and anything generated by suspense

In "Sync" state, we abort our "idle wait" future, and resolve the sync queue immediately and escape. Because we completed
work before the next rAF, any edits can be immediately processed before the frame ends. Generally though, we want to leave
as much time to rAF as possible. "Sync" is currently only used by onInput - we'll leave some docs telling people not to
do anything too arduous from onInput.

For the rest, we defer to the rIC period and work down each queue from high to low.
*/

/// The scheduler holds basically everything around "working"
///
/// Each scope has the ability to lightly interact with the scheduler (IE, schedule an update) but ultimately the scheduler calls the components.
///
/// In Dioxus, the scheduler provides 4 priority levels - each with their own "DiffMachine". The DiffMachine state can be saved if the deadline runs
/// out.
///
/// Saved DiffMachine state can be self-referential, so we need to be careful about how we save it. All self-referential data is a link between
/// pending DiffInstructions, Mutations, and their underlying Scope. It's okay for us to be self-referential with this data, provided we don't priority
/// task shift to a higher priority task that needs mutable access to the same scopes.
///
/// We can prevent this safety issue from occurring if we track which scopes are invalidated when starting a new task.
///
/// There's a lot of raw pointers here...
///
/// Since we're building self-referential structures for each component, we need to make sure that the referencs stay stable
/// The best way to do that is a bump allocator.
///
///
///
impl VirtualDom {
    // returns true if the event is discrete
    pub fn handle_ui_event(&mut self, event: UserEvent) -> bool {
        let (discrete, priority) = event_meta(&event);

        if let Some(scope) = self.get_scope_mut(&event.scope) {
            if let Some(element) = event.mounted_dom_id {
                // TODO: bubble properly here
                scope.call_listener(event, element);

                while let Ok(Some(dirty_scope)) = self.receiver.try_next() {
                    //
                    //     self.add_dirty_scope(dirty_scope, trigger.priority)
                }
            }
        }

        // use EventPriority::*;

        // match priority {
        //     Immediate => todo!(),
        //     High => todo!(),
        //     Medium => todo!(),
        //     Low => todo!(),
        // }

        discrete
    }

    fn prepare_work(&mut self) {
        // while let Some(trigger) = self.ui_events.pop_back() {
        //     if let Some(scope) = self.get_scope_mut(&trigger.scope) {}
        // }
    }

    // nothing to do, no events on channels, no work
    pub fn has_any_work(&self) -> bool {
        !(self.dirty_scopes.is_empty() && self.ui_events.is_empty())
    }

    /// re-balance the work lanes, ensuring high-priority work properly bumps away low priority work
    fn balance_lanes(&mut self) {}

    fn save_work(&mut self, lane: SavedDiffWork) {
        let saved: SavedDiffWork<'static> = unsafe { std::mem::transmute(lane) };
        self.saved_state = Some(saved);
    }

    unsafe fn load_work(&mut self) -> SavedDiffWork<'static> {
        self.saved_state.take().unwrap().extend()
    }

    pub fn handle_channel_msg(&mut self, msg: SchedulerMsg) {
        match msg {
            SchedulerMsg::Immediate(_) => todo!(),

            SchedulerMsg::UiEvent(event) => {
                //

                let (discrete, priority) = event_meta(&event);

                if let Some(scope) = self.get_scope_mut(&event.scope) {
                    if let Some(element) = event.mounted_dom_id {
                        // TODO: bubble properly here
                        scope.call_listener(event, element);

                        while let Ok(Some(dirty_scope)) = self.receiver.try_next() {
                            //
                            //     self.add_dirty_scope(dirty_scope, trigger.priority)
                        }
                    }
                }

                discrete;
            }
        }
    }

    /// Load the current lane, and work on it, periodically checking in if the deadline has been reached.
    ///
    /// Returns true if the lane is finished before the deadline could be met.
    pub fn work_on_current_lane(
        &mut self,
        deadline_reached: impl FnMut() -> bool,
        mutations: &mut Vec<Mutations>,
    ) -> bool {
        // Work through the current subtree, and commit the results when it finishes
        // When the deadline expires, give back the work
        let saved_state = unsafe { self.load_work() };

        // We have to split away some parts of ourself - current lane is borrowed mutably
        let shared = self.clone();
        let mut machine = unsafe { saved_state.promote(&shared) };

        let mut ran_scopes = FxHashSet::default();

        if machine.stack.is_empty() {
            let shared = self.clone();

            self.dirty_scopes
                .retain(|id| shared.get_scope(id).is_some());
            self.dirty_scopes.sort_by(|a, b| {
                let h1 = shared.get_scope(a).unwrap().height;
                let h2 = shared.get_scope(b).unwrap().height;
                h1.cmp(&h2).reverse()
            });

            if let Some(scopeid) = self.dirty_scopes.pop() {
                log::info!("handling dirty scope {:?}", scopeid);
                if !ran_scopes.contains(&scopeid) {
                    ran_scopes.insert(scopeid);
                    log::debug!("about to run scope {:?}", scopeid);

                    if let Some(component) = self.get_scope_mut(&scopeid) {
                        if component.run_scope(&self) {
                            let (old, new) =
                                (component.frames.wip_head(), component.frames.fin_head());
                            // let (old, new) = (component.frames.wip_head(), component.frames.fin_head());
                            machine.stack.scope_stack.push(scopeid);
                            machine.stack.push(DiffInstruction::Diff { new, old });
                        }
                    }
                }
            }
        }

        let work_completed = machine.work(deadline_reached);

        // log::debug!("raw edits {:?}", machine.mutations.edits);

        let mut machine: DiffMachine<'static> = unsafe { std::mem::transmute(machine) };
        // let mut saved = machine.save();

        if work_completed {
            for node in machine.seen_scopes.drain() {
                // self.dirty_scopes.clear();
                // self.ui_events.clear();
                self.dirty_scopes.remove(&node);
                // self.dirty_scopes.remove(&node);
            }

            let mut new_mutations = Mutations::new();

            for edit in machine.mutations.edits.drain(..) {
                new_mutations.edits.push(edit);
            }

            // for edit in saved.edits.drain(..) {
            //     new_mutations.edits.push(edit);
            // }

            // std::mem::swap(&mut new_mutations, &mut saved.mutations);

            mutations.push(new_mutations);

            // log::debug!("saved edits {:?}", mutations);

            let mut saved = machine.save();
            self.save_work(saved);
            true

            // self.save_work(saved);
            // false
        } else {
            false
        }
    }

    /// The primary workhorse of the VirtualDOM.
    ///
    /// Uses some fairly complex logic to schedule what work should be produced.
    ///
    /// Returns a list of successful mutations.
    pub fn work_with_deadline<'a>(
        &'a mut self,
        mut deadline: impl FnMut() -> bool,
    ) -> Vec<Mutations<'a>> {
        /*
        Strategy:
        - When called, check for any UI events that might've been received since the last frame.
        - Dump all UI events into a "pending discrete" queue and a "pending continuous" queue.

        - If there are any pending discrete events, then elevate our priority level. If our priority level is already "high,"
            then we need to finish the high priority work first. If the current work is "low" then analyze what scopes
            will be invalidated by this new work. If this interferes with any in-flight medium or low work, then we need
            to bump the other work out of the way, or choose to process it so we don't have any conflicts.
            'static components have a leg up here since their work can be re-used among multiple scopes.
            "High priority" is only for blocking! Should only be used on "clicks"

        - If there are no pending discrete events, then check for continuous events. These can be completely batched

        - we batch completely until we run into a discrete event
        - all continuous events are batched together
        - so D C C C C C would be two separate events - D and C. IE onclick and onscroll
        - D C C C C C C D C C C D would be D C D C D in 5 distinct phases.

        - !listener bubbling is not currently implemented properly and will need to be implemented somehow in the future
            - we need to keep track of element parents to be able to traverse properly


        Open questions:
        - what if we get two clicks from the component during the same slice?
            - should we batch?
            - react says no - they are continuous
            - but if we received both - then we don't need to diff, do we? run as many as we can and then finally diff?
        */
        let mut committed_mutations = Vec::<Mutations<'static>>::new();

        while self.has_any_work() {
            while let Ok(Some(msg)) = self.receiver.try_next() {
                match msg {
                    SchedulerMsg::Immediate(im) => {
                        self.dirty_scopes.insert(im);
                    }
                    SchedulerMsg::UiEvent(evt) => {
                        self.ui_events.push_back(evt);
                    }
                }
            }

            // switch our priority, pop off any work
            while let Some(event) = self.ui_events.pop_front() {
                if let Some(scope) = self.get_scope_mut(&event.scope) {
                    if let Some(element) = event.mounted_dom_id {
                        log::info!("Calling listener {:?}, {:?}", event.scope, element);

                        // TODO: bubble properly here
                        scope.call_listener(event, element);

                        while let Ok(Some(dirty_scope)) = self.receiver.try_next() {
                            match dirty_scope {
                                SchedulerMsg::Immediate(im) => {
                                    self.dirty_scopes.insert(im);
                                }
                                SchedulerMsg::UiEvent(e) => self.ui_events.push_back(e),
                            }
                        }
                    }
                }
            }

            let work_complete = self.work_on_current_lane(&mut deadline, &mut committed_mutations);

            if !work_complete {
                return committed_mutations;
            }
        }

        committed_mutations
    }

    /// Work the scheduler down, not polling any ongoing tasks.
    ///
    /// Will use the standard priority-based scheduling, batching, etc, but just won't interact with the async reactor.
    pub fn work_sync<'a>(&'a mut self) -> Vec<Mutations<'a>> {
        let mut committed_mutations = Vec::new();

        while let Ok(Some(msg)) = self.receiver.try_next() {
            self.handle_channel_msg(msg);
        }

        if !self.has_any_work() {
            return committed_mutations;
        }

        while self.has_any_work() {
            self.prepare_work();
            self.work_on_current_lane(|| false, &mut committed_mutations);
        }

        committed_mutations
    }

    /// Restart the entire VirtualDOM from scratch, wiping away any old state and components.
    ///
    /// Typically used to kickstart the VirtualDOM after initialization.
    pub fn rebuild(&mut self, base_scope: ScopeId) -> Mutations {
        let mut shared = self.clone();
        let mut diff_machine = DiffMachine::new(Mutations::new(), &mut shared);

        // TODO: drain any in-flight work
        let cur_component = self
            .pool
            .get_scope_mut(&base_scope)
            .expect("The base scope should never be moved");

        log::debug!("rebuild {:?}", base_scope);

        // We run the component. If it succeeds, then we can diff it and add the changes to the dom.
        if cur_component.run_scope(&self) {
            diff_machine
                .stack
                .create_node(cur_component.frames.fin_head(), MountType::Append);

            diff_machine.stack.scope_stack.push(base_scope);

            diff_machine.work(|| false);
        } else {
            // todo: should this be a hard error?
            log::warn!(
                "Component failed to run successfully during rebuild.
                This does not result in a failed rebuild, but indicates a logic failure within your app."
            );
        }

        unsafe { std::mem::transmute(diff_machine.mutations) }
    }

    pub fn hard_diff(&mut self, base_scope: ScopeId) -> Mutations {
        let cur_component = self
            .pool
            .get_scope_mut(&base_scope)
            .expect("The base scope should never be moved");

        log::debug!("hard diff {:?}", base_scope);

        if cur_component.run_scope(&self) {
            let mut diff_machine = DiffMachine::new(Mutations::new(), &mut self);
            diff_machine.cfg.force_diff = true;
            diff_machine.diff_scope(base_scope);
            diff_machine.mutations
        } else {
            Mutations::new()
        }
    }
}

impl Future for VirtualDom {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut all_pending = true;

        for fut in self.pending_futures.iter() {
            let scope = self
                .pool
                .get_scope_mut(&fut)
                .expect("Scope should never be moved");

            let items = scope.items.get_mut();
            for task in items.tasks.iter_mut() {
                let t = task.as_mut();
                let g = unsafe { Pin::new_unchecked(t) };
                match g.poll(cx) {
                    Poll::Ready(r) => {
                        all_pending = false;
                    }
                    Poll::Pending => {}
                }
            }
        }

        match all_pending {
            true => Poll::Pending,
            false => Poll::Ready(()),
        }
    }
}
