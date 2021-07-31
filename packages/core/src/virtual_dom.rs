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

use futures_util::StreamExt;

use crate::hooks::{SuspendedContext, SuspenseHook};
use crate::{arena::SharedResources, innerlude::*};

use std::any::Any;

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
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

    pub triggers: RefCell<Vec<EventTrigger>>,

    // for managing the props that were used to create the dom
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,

    #[doc(hidden)]
    _root_props: std::pin::Pin<Box<dyn std::any::Any>>,
}

// ======================================
// Public Methods for the VirtualDom
// ======================================
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
            triggers: Default::default(),
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    pub fn launch_in_place(root: FC<()>) -> Self {
        let mut s = Self::new(root);
        s.rebuild_in_place();
        s
    }

    /// Creates a new virtualdom and immediately rebuilds it in place, not caring about the RealDom to write into.
    ///
    pub fn launch_with_props_in_place<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let mut s = Self::new_with_props(root, root_props);
        s.rebuild_in_place();
        s
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
        let mut realdom = DebugDom::new();
        let mut edits = Vec::new();
        self.rebuild(&mut realdom, &mut edits)?;
        Ok(edits)
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom rom scratch
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application
    pub fn rebuild<'s>(
        &'s mut self,
        realdom: &'_ mut dyn RealDom<'s>,
        edits: &'_ mut Vec<DomEdit<'s>>,
    ) -> Result<()> {
        let mut diff_machine = DiffMachine::new(edits, realdom, self.base_scope, &self.shared);

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

        Ok(())
    }

    ///
    ///
    ///
    ///
    ///
    pub fn queue_event(&self, trigger: EventTrigger) {
        let mut triggers = self.triggers.borrow_mut();
        triggers.push(trigger);
    }

    /// This method is the most sophisticated way of updating the virtual dom after an external event has been triggered.
    ///  
    /// Given a synthetic event, the component that triggered the event, and the index of the callback, this runs the virtual
    /// dom to completion, tagging components that need updates, compressing events together, and finally emitting a single
    /// change list.
    ///
    /// If implementing an external renderer, this is the perfect method to combine with an async event loop that waits on
    /// listeners, something like this:
    ///
    /// ```ignore
    /// while let Ok(event) = receiver.recv().await {
    ///     let edits = self.internal_dom.progress_with_event(event)?;
    ///     for edit in &edits {
    ///         patch_machine.handle_edit(edit);
    ///     }
    /// }
    /// ```
    ///
    /// Note: this method is not async and does not provide suspense-like functionality. It is up to the renderer to provide the
    /// executor and handlers for suspense as show in the example.
    ///
    /// ```ignore
    /// let (sender, receiver) = channel::new();
    /// sender.send(EventTrigger::start());
    ///
    /// let mut dom = VirtualDom::new();
    /// dom.suspense_handler(|event| sender.send(event));
    ///
    /// while let Ok(diffs) = dom.progress_with_event(receiver.recv().await) {
    ///     render(diffs);
    /// }
    ///
    /// ```
    //
    // Developer notes:
    // ----
    // This method has some pretty complex safety guarantees to uphold.
    // We interact with bump arenas, raw pointers, and use UnsafeCell to get a partial borrow of the arena.
    // The final EditList has edits that pull directly from the Bump Arenas which add significant complexity
    // in crafting a 100% safe solution with traditional lifetimes. Consider this method to be internally unsafe
    // but the guarantees provide a safe, fast, and efficient abstraction for the VirtualDOM updating framework.
    //
    // A good project would be to remove all unsafe from this crate and move the unsafety into safer abstractions.
    pub async fn progress_with_event<'a, 's>(
        &'s mut self,
        realdom: &'a mut dyn RealDom<'s>,
        edits: &'a mut Vec<DomEdit<'s>>,
    ) -> Result<()> {
        let trigger = self.triggers.borrow_mut().pop().expect("failed");

        let mut diff_machine = DiffMachine::new(edits, realdom, trigger.originator, &self.shared);

        match &trigger.event {
            // When a scope gets destroyed during a diff, it gets its own garbage collection event
            // However, an old scope might be attached
            VirtualEvent::GarbageCollection => {
                let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

                let mut garbage_list = scope.consume_garbage();

                while let Some(node) = garbage_list.pop() {
                    match &node.kind {
                        VNodeKind::Text(_) => {
                            //
                            self.shared.collect_garbage(node.direct_id())
                        }
                        VNodeKind::Anchor(anchor) => {
                            //
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
                            // run the destructors
                            todo!();
                        }
                        VNodeKind::Suspended(node) => {
                            // make sure the task goes away
                            todo!();
                        }
                    }
                }
            }

            // Nothing yet
            VirtualEvent::AsyncEvent { .. } => {}

            // Suspense Events! A component's suspended node is updated
            VirtualEvent::SuspenseEvent { hook_idx, domnode } => {
                // Safety: this handler is the only thing that can mutate shared items at this moment in tim
                let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

                // safety: we are sure that there are no other references to the inner content of suspense hooks
                let hook = unsafe { scope.hooks.get_mut::<SuspenseHook>(*hook_idx) }.unwrap();

                let cx = Context { scope, props: &() };
                let scx = SuspendedContext { inner: cx };

                // generate the new node!
                let nodes: Option<VNode<'s>> = (&hook.callback)(scx);
                match nodes {
                    None => {
                        log::warn!("Suspense event came through, but there was no mounted node to update >:(");
                    }
                    Some(nodes) => {
                        // todo!("using the wrong frame");
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

            // This is the "meat" of our cooperative scheduler
            // As updates flow in, we re-evalute the event queue and decide if we should be switching the type of work
            //
            // We use the reconciler to request new IDs and then commit/uncommit the IDs when the scheduler is finished
            _ => {
                diff_machine
                    .get_scope_mut(&trigger.originator)
                    .map(|f| f.call_listener(trigger));

                // Now, there are events in the queue
                let mut updates = self.shared.borrow_queue();

                // Order the nodes by their height, we want the nodes with the smallest depth on top
                // This prevents us from running the same component multiple times
                updates.sort_unstable();

                log::debug!("There are: {:#?} updates to be processed", updates.len());

                // Iterate through the triggered nodes (sorted by height) and begin to diff them
                for update in updates.drain(..) {
                    log::debug!("Running updates for: {:#?}", update);

                    // Make sure this isn't a node we've already seen, we don't want to double-render anything
                    // If we double-renderer something, this would cause memory safety issues
                    if diff_machine.seen_scopes.contains(&update.idx) {
                        log::debug!("Skipping update for: {:#?}", update);
                        continue;
                    }

                    // Start a new mutable borrow to components
                    // We are guaranteeed that this scope is unique because we are tracking which nodes have modified in the diff machine
                    let cur_component = diff_machine
                        .get_scope_mut(&update.idx)
                        .expect("Failed to find scope or borrow would be aliasing");

                    // Now, all the "seen nodes" are nodes that got notified by running this listener
                    diff_machine.seen_scopes.insert(update.idx.clone());

                    if cur_component.run_scope().is_ok() {
                        let (old, new) = (
                            cur_component.frames.wip_head(),
                            cur_component.frames.fin_head(),
                        );
                        diff_machine.diff_node(old, new);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn wait_for_event(&mut self) -> Option<EventTrigger> {
        let r = self.shared.tasks.clone();
        let mut r = r.borrow_mut();
        let gh = r.next().await;

        gh
    }

    pub fn any_pending_events(&self) -> bool {
        let r = self.shared.tasks.clone();
        let r = r.borrow();
        !r.is_empty()
    }

    pub fn base_scope(&self) -> &Scope {
        unsafe { self.shared.get_scope(self.base_scope).unwrap() }
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        unsafe { self.shared.get_scope(id) }
    }
}

// TODO!
// These impls are actually wrong. The DOM needs to have a mutex implemented.
unsafe impl Sync for VirtualDom {}
unsafe impl Send for VirtualDom {}
