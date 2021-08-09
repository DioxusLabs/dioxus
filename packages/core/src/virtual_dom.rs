//! # VirtualDOM Implementation for Rust
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
//! In this file, multiple items are defined. This file is big, but should be documented well to
//! navigate the innerworkings of the Dom. We try to keep these main mechanics in this file to limit
//! the possible exposed API surface (keep fields private). This particular implementation of VDOM
//! is extremely efficient, but relies on some unsafety under the hood to do things like manage
//! micro-heaps for components. We are currently working on refactoring the safety out into safe(r)
//! abstractions, but current tests (MIRI and otherwise) show no issues with the current implementation.
//!
//! Included is:
//! - The [`VirtualDom`] itself
//! - The [`Scope`] object for mangning component lifecycle
//! - The [`ActiveFrame`] object for managing the Scope`s microheap
//! - The [`Context`] object for exposing VirtualDOM API to components
//! - The [`NodeFactory`] object for lazyily exposing the `Context` API to the nodebuilder API
//! - The [`Hook`] object for exposing state management in components.
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.
#![allow(unreachable_code)]
use futures_util::StreamExt;
use fxhash::FxHashMap;

use crate::hooks::{SuspendedContext, SuspenseHook};
use crate::{arena::SharedResources, innerlude::*};

use std::any::Any;

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet};
use std::pin::Pin;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
///
///
///
///
///
///
///
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    pub shared: SharedResources,

    /// The index of the root component
    /// Should always be the first (gen=0, id=0)
    pub base_scope: ScopeId,

    active_fibers: Vec<Fiber<'static>>,

    // for managing the props that were used to create the dom
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,

    #[doc(hidden)]
    _root_props: std::pin::Pin<Box<dyn std::any::Any>>,
}

impl VirtualDom {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    ///
    /// As an end-user, you'll want to use the Renderer's "new" method instead of this method.
    /// Directly creating the VirtualDOM is only useful when implementing a new renderer.
    ///
    ///
    /// ```ignore
    /// // Directly from a closure
    ///
    /// let dom = VirtualDom::new(|cx| cx.render(rsx!{ div {"hello world"} }));
    ///
    /// // or pass in...
    ///
    /// let root = |cx| {
    ///     cx.render(rsx!{
    ///         div {"hello world"}
    ///     })
    /// }
    /// let dom = VirtualDom::new(root);
    ///
    /// // or directly from a fn
    ///
    /// fn Example(cx: Context<()>) -> DomTree  {
    ///     cx.render(rsx!{ div{"hello world"} })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Start a new VirtualDom instance with a dependent cx.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    /// ```ignore
    /// // Directly from a closure
    ///
    /// let dom = VirtualDom::new(|cx| cx.render(rsx!{ div {"hello world"} }));
    ///
    /// // or pass in...
    ///
    /// let root = |cx| {
    ///     cx.render(rsx!{
    ///         div {"hello world"}
    ///     })
    /// }
    /// let dom = VirtualDom::new(root);
    ///
    /// // or directly from a fn
    ///
    /// fn Example(cx: Context, props: &SomeProps) -> VNode  {
    ///     cx.render(rsx!{ div{"hello world"} })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let components = SharedResources::new();

        let root_props: Pin<Box<dyn Any>> = Box::pin(root_props);
        let props_ptr = root_props.as_ref().downcast_ref::<P>().unwrap() as *const P;

        let link = components.clone();

        let base_scope = components.insert_scope_with_key(move |myidx| {
            let caller = NodeFactory::create_component_caller(root, props_ptr as *const _);
            Scope::new(caller, myidx, None, 0, ScopeChildren(&[]), link)
        });

        Self {
            base_scope,
            _root_props: root_props,
            shared: components,
            active_fibers: Vec::new(),
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    pub fn launch_in_place(root: FC<()>) -> Self {
        let mut s = Self::new(root);
        s.rebuild_in_place().unwrap();
        s
    }

    /// Creates a new virtualdom and immediately rebuilds it in place, not caring about the RealDom to write into.
    ///
    pub fn launch_with_props_in_place<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let mut s = Self::new_with_props(root, root_props);
        s.rebuild_in_place().unwrap();
        s
    }

    pub fn base_scope(&self) -> &Scope {
        unsafe { self.shared.get_scope(self.base_scope).unwrap() }
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        unsafe { self.shared.get_scope(id) }
    }

    /// Rebuilds the VirtualDOM from scratch, but uses a "dummy" RealDom.
    ///
    /// Used in contexts where a real copy of the  structure doesn't matter, and the VirtualDOM is the source of truth.
    ///
    /// ## Why?
    ///
    /// This method uses the `DebugDom` under the hood - essentially making the VirtualDOM's diffing patches a "no-op".
    ///
    /// SSR takes advantage of this by using Dioxus itself as the source of truth, and rendering from the tree directly.
    pub fn rebuild_in_place(&mut self) -> Result<Vec<DomEdit>> {
        todo!();
        // let mut realdom = DebugDom::new();
        // let mut edits = Vec::new();
        // self.rebuild(&mut realdom, &mut edits)?;
        // Ok(edits)
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom rom scratch
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application
    ///
    /// Events like garabge collection, application of refs, etc are not handled by this method and can only be progressed
    /// through "run"
    ///
    pub fn rebuild<'s>(&'s mut self) -> Result<Vec<DomEdit<'s>>> {
        let mut edits = Vec::new();
        let mutations = Mutations { edits: Vec::new() };
        let mut diff_machine = DiffMachine::new(mutations, self.base_scope, &self.shared);

        let cur_component = diff_machine
            .get_scope_mut(&self.base_scope)
            .expect("The base scope should never be moved");

        // We run the component. If it succeeds, then we can diff it and add the changes to the dom.
        if cur_component.run_scope().is_ok() {
            let meta = diff_machine.create_vnode(cur_component.frames.fin_head());
            diff_machine.edit_append_children(meta.added_to_stack);
        } else {
            // todo: should this be a hard error?
            log::warn!(
                "Component failed to run succesfully during rebuild.
                This does not result in a failed rebuild, but indicates a logic failure within your app."
            );
        }

        Ok(edits)
    }

    // async fn select_next_event(&mut self) -> Option<EventTrigger> {
    //     let mut receiver = self.shared.task_receiver.borrow_mut();

    //     // drain the in-flight events so that we can sort them out with the current events
    //     while let Ok(Some(trigger)) = receiver.try_next() {
    //         log::info!("retrieving event from receiver");
    //         let key = self.shared.make_trigger_key(&trigger);
    //         self.pending_events.insert(key, trigger);
    //     }

    //     if self.pending_events.is_empty() {
    //         // Continuously poll the future pool and the event receiver for work
    //         let mut tasks = self.shared.async_tasks.borrow_mut();
    //         let tasks_tasks = tasks.next();

    //         let mut receiver = self.shared.task_receiver.borrow_mut();
    //         let reciv_task = receiver.next();

    //         futures_util::pin_mut!(tasks_tasks);
    //         futures_util::pin_mut!(reciv_task);

    //         let trigger = match futures_util::future::select(tasks_tasks, reciv_task).await {
    //             futures_util::future::Either::Left((trigger, _)) => trigger,
    //             futures_util::future::Either::Right((trigger, _)) => trigger,
    //         }
    //         .unwrap();
    //         let key = self.shared.make_trigger_key(&trigger);
    //         self.pending_events.insert(key, trigger);
    //     }

    //     // pop the most important event off
    //     let key = self.pending_events.keys().next().unwrap().clone();
    //     let trigger = self.pending_events.remove(&key).unwrap();

    //     Some(trigger)
    // }

    /// Runs the virtualdom immediately, not waiting for any suspended nodes to complete.
    ///
    /// This method will not wait for any suspended tasks, completely skipping over
    pub fn run_immediate<'s>(&'s mut self) -> Result<Mutations<'s>> {
        //

        todo!()
    }

    /// Runs the virtualdom with no time limit.
    ///
    /// If there are pending tasks, they will be progressed before returning. This is useful when rendering an application
    /// that has suspended nodes or suspended tasks. Be warned - any async tasks running forever will prevent this method
    /// from completing. Consider using `run` and specifing a deadline.
    pub async fn run_unbounded<'s>(&'s mut self) -> Result<Mutations<'s>> {
        self.run_with_deadline(|| false).await
    }

    /// Run the virtualdom with a time limit.
    ///
    /// This method will progress async tasks until the deadline is reached. If tasks are completed before the deadline,
    /// and no tasks are pending, this method will return immediately. If tasks are still pending, then this method will
    /// exhaust the deadline working on them.
    ///
    /// This method is useful when needing to schedule the virtualdom around other tasks on the main thread to prevent
    /// "jank". It will try to finish whatever work it has by the deadline to free up time for other work.
    ///
    /// Due to platform differences in how time is handled, this method accepts a closure that must return true when the
    /// deadline is exceeded. However, the deadline won't be met precisely, so you might want to build some wiggle room
    /// into the deadline closure manually.
    ///
    /// The deadline is checked before starting to diff components. This strikes a balance between the overhead of checking
    /// the deadline and just completing the work. However, if an individual component takes more than 16ms to render, then
    /// the screen will "jank" up. In debug, this will trigger an alert.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut dom = VirtualDom::new(|cx| cx.render(rsx!( div {"hello"} )));
    /// loop {
    ///     let started = std::time::Instant::now();
    ///     let deadline = move || std::time::Instant::now() - started > std::time::Duration::from_millis(16);
    ///     
    ///     let mutations = dom.run_with_deadline(deadline).await;
    ///     apply_mutations(mutations);
    /// }
    /// ```
    pub async fn run_with_deadline<'s>(
        &'s mut self,
        mut deadline_exceeded: impl FnMut() -> bool,
    ) -> Result<Mutations<'s>> {
        let cur_component = self.base_scope;

        let mut diff_machine =
            DiffMachine::new(Mutations { edits: Vec::new() }, cur_component, &self.shared);

        /*
        Strategy:
        1. Check if there are any events in the receiver.
        2. If there are, process them and create a new fiber.
        3. If there are no events, then choose a fiber to work on.
        4. If there are no fibers, then wait for the next event from the receiver.
        5. While processing a fiber, periodically check if we're out of time
        6. If we are almost out of time, then commit our edits to the realdom
        7. Whenever a fiber is finished, immediately commit it. (IE so deadlines can be infinite if unsupported)
        */

        // 1. Consume any pending events and create new fibers
        let mut receiver = self.shared.task_receiver.borrow_mut();
        while let Ok(Some(trigger)) = receiver.try_next() {
            // todo: cache the fibers
            let mut fiber = Fiber::new();

            match &trigger.event {
                // If any input event is received, then we need to create a new fiber
                VirtualEvent::ClipboardEvent(_)
                | VirtualEvent::CompositionEvent(_)
                | VirtualEvent::KeyboardEvent(_)
                | VirtualEvent::FocusEvent(_)
                | VirtualEvent::FormEvent(_)
                | VirtualEvent::SelectionEvent(_)
                | VirtualEvent::TouchEvent(_)
                | VirtualEvent::UIEvent(_)
                | VirtualEvent::WheelEvent(_)
                | VirtualEvent::MediaEvent(_)
                | VirtualEvent::AnimationEvent(_)
                | VirtualEvent::TransitionEvent(_)
                | VirtualEvent::ToggleEvent(_)
                | VirtualEvent::MouseEvent(_)
                | VirtualEvent::PointerEvent(_) => {
                    if let Some(scope) = self.shared.get_scope_mut(trigger.originator) {
                        scope.call_listener(trigger)?;
                    }
                }

                VirtualEvent::AsyncEvent { .. } => {
                    while let Ok(Some(event)) = receiver.try_next() {
                        fiber.pending_scopes.push(event.originator);
                    }
                }

                // These shouldn't normally be received, but if they are, it's done because some task set state manually
                // Instead of batching the results,
                VirtualEvent::ScheduledUpdate { height: u32 } => {}

                // Suspense Events! A component's suspended node is updated
                VirtualEvent::SuspenseEvent { hook_idx, domnode } => {
                    // Safety: this handler is the only thing that can mutate shared items at this moment in tim
                    let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

                    // safety: we are sure that there are no other references to the inner content of suspense hooks
                    let hook = unsafe { scope.hooks.get_mut::<SuspenseHook>(*hook_idx) }.unwrap();

                    let cx = Context { scope, props: &() };
                    let scx = SuspendedContext { inner: cx };

                    // generate the new node!
                    let nodes: Option<VNode> = (&hook.callback)(scx);
                    match nodes {
                        None => {
                            log::warn!(
                                "Suspense event came through, but there were no generated nodes >:(."
                            );
                        }
                        Some(nodes) => {
                            // allocate inside the finished frame - not the WIP frame
                            let nodes = scope.frames.finished_frame().bump.alloc(nodes);

                            // push the old node's root onto the stack
                            let real_id = domnode.get().ok_or(Error::NotMounted)?;
                            diff_machine.edit_push_root(real_id);

                            // push these new nodes onto the diff machines stack
                            let meta = diff_machine.create_vnode(&*nodes);

                            // replace the placeholder with the new nodes we just pushed on the stack
                            diff_machine.edit_replace_with(1, meta.added_to_stack);
                        }
                    }
                }

                // Collecting garabge is not currently interruptible.
                //
                // In the future, it could be though
                VirtualEvent::GarbageCollection => {
                    let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

                    let mut garbage_list = scope.consume_garbage();

                    let mut scopes_to_kill = Vec::new();
                    while let Some(node) = garbage_list.pop() {
                        match &node.kind {
                            VNodeKind::Text(_) => {
                                self.shared.collect_garbage(node.direct_id());
                            }
                            VNodeKind::Anchor(_) => {
                                self.shared.collect_garbage(node.direct_id());
                            }
                            VNodeKind::Suspended(_) => {
                                self.shared.collect_garbage(node.direct_id());
                            }

                            VNodeKind::Element(el) => {
                                self.shared.collect_garbage(node.direct_id());
                                for child in el.children {
                                    garbage_list.push(child);
                                }
                            }

                            VNodeKind::Fragment(frag) => {
                                for child in frag.children {
                                    garbage_list.push(child);
                                }
                            }

                            VNodeKind::Component(comp) => {
                                // TODO: run the hook destructors and then even delete the scope

                                let scope_id = comp.ass_scope.get().unwrap();
                                let scope = self.get_scope(scope_id).unwrap();
                                let root = scope.root();
                                garbage_list.push(root);
                                scopes_to_kill.push(scope_id);
                            }
                        }
                    }

                    for scope in scopes_to_kill {
                        // oy kill em
                        log::debug!("should be removing scope {:#?}", scope);
                    }
                }
            }
        }

        while !deadline_exceeded() {
            let mut receiver = self.shared.task_receiver.borrow_mut();

            // no messages to receive, just work on the fiber
        }

        Ok(diff_machine.edits)
    }

    pub fn get_event_sender(&self) -> futures_channel::mpsc::UnboundedSender<EventTrigger> {
        self.shared.task_sender.clone()
    }

    fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        unsafe { self.shared.get_scope_mut(id) }
    }
}

// TODO!
// These impls are actually wrong. The DOM needs to have a mutex implemented.
unsafe impl Sync for VirtualDom {}
unsafe impl Send for VirtualDom {}

struct Fiber<'a> {
    // scopes that haven't been updated yet
    pending_scopes: Vec<ScopeId>,

    pending_nodes: Vec<*const VNode<'a>>,

    // WIP edits
    edits: Vec<DomEdit<'a>>,

    started: bool,

    completed: bool,
}

impl Fiber<'_> {
    fn new() -> Self {
        Self {
            pending_scopes: Vec::new(),
            pending_nodes: Vec::new(),
            edits: Vec::new(),
            started: false,
            completed: false,
        }
    }
}

/// The "Mutations" object holds the changes that need to be made to the DOM.
pub struct Mutations<'s> {
    // todo: apply node refs
    // todo: apply effects
    pub edits: Vec<DomEdit<'s>>,
}
