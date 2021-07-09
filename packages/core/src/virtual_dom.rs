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

use crate::hooklist::HookList;
use crate::{arena::ScopeArena, innerlude::*};
use appendlist::AppendList;
use bumpalo::Bump;
use futures::FutureExt;
use slotmap::DefaultKey;
use slotmap::SlotMap;
use std::marker::PhantomData;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};

pub type ScopeIdx = DefaultKey;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    pub components: ScopeArena,

    /// The index of the root component
    /// Should always be the first (gen=0, id=0)
    pub base_scope: ScopeIdx,

    /// All components dump their updates into a queue to be processed
    pub(crate) event_queue: EventQueue,

    /// a strong allocation to the "caller" for the original component and its props
    #[doc(hidden)]
    _root_caller: Rc<OpaqueComponent>,
    // _root_caller: Rc<OpaqueComponent<'static>>,
    /// Type of the original cx. This is stored as TypeId so VirtualDom does not need to be generic.
    ///
    /// Whenver props need to be updated, an Error will be thrown if the new props do not
    /// match the props used to create the VirtualDom.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,
}

/// The `RealDomNode` is an ID handle that corresponds to a foreign DOM node.
///
/// "u64" was chosen for two reasons
/// - 0 cost hashing
/// - use with slotmap and other versioned slot arenas
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealDomNode(pub u64);
impl RealDomNode {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    pub fn empty() -> Self {
        Self(u64::MIN)
    }
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
    /// fn Example(cx: Context<()>) -> VNode  {
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
        let components = ScopeArena::new(SlotMap::new());

        // Normally, a component would be passed as a child in the RSX macro which automatically produces OpaqueComponents
        // Here, we need to make it manually, using an RC to force the Weak reference to stick around for the main scope.
        let _root_caller: Rc<OpaqueComponent> = Rc::new(move |scope| {
            // let _root_caller: Rc<OpaqueComponent<'static>> = Rc::new(move |scope| {
            // the lifetime of this closure is just as long as the lifetime on the scope reference
            // this closure moves root props (which is static) into this closure
            let props = unsafe { &*(&root_props as *const _) };
            root(Context {
                props,
                scope,
                tasks: todo!(),
            })
        });

        // Create a weak reference to the OpaqueComponent for the root scope to use as its render function
        let caller_ref = Rc::downgrade(&_root_caller);

        // Build a funnel for hooks to send their updates into. The `use_hook` method will call into the update funnel.
        let event_queue = EventQueue::default();
        let _event_queue = event_queue.clone();

        // Make the first scope
        // We don't run the component though, so renderers will need to call "rebuild" when they initialize their DOM
        let link = components.clone();

        let base_scope = components
            .with(|arena| {
                arena.insert_with_key(move |myidx| {
                    let event_channel = _event_queue.new_channel(0, myidx);
                    Scope::new(caller_ref, myidx, None, 0, event_channel, link, &[])
                })
            })
            .unwrap();

        Self {
            _root_caller,
            base_scope,
            event_queue,
            components,
            _root_prop_type: TypeId::of::<P>(),
        }
    }
}

// ======================================
// Private Methods for the VirtualDom
// ======================================
impl VirtualDom {
    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom rom scratch
    /// Currently this doesn't do what we want it to do
    pub fn rebuild<'s, Dom: RealDom<'s>>(&'s mut self, realdom: &mut Dom) -> Result<()> {
        let mut diff_machine = DiffMachine::new(
            realdom,
            &self.components,
            self.base_scope,
            self.event_queue.clone(),
        );

        // Schedule an update and then immediately call it on the root component
        // This is akin to a hook being called from a listener and requring a re-render
        // Instead, this is done on top-level component
        let base = self.components.try_get(self.base_scope)?;

        let update = &base.event_channel;
        update();

        self.progress_completely(&mut diff_machine)?;

        Ok(())
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
    pub fn progress_with_event<'s, Dom: RealDom<'s>>(
        &'s mut self,
        realdom: &'_ mut Dom,
        trigger: EventTrigger,
    ) -> Result<()> {
        let id = trigger.component_id.clone();

        self.components.try_get_mut(id)?.call_listener(trigger)?;

        let mut diff_machine =
            DiffMachine::new(realdom, &self.components, id, self.event_queue.clone());

        self.progress_completely(&mut diff_machine)?;

        Ok(())
    }

    /// Consume the event queue, descending depth-first.
    /// Only ever run each component once.
    ///
    /// The DiffMachine logs its progress as it goes which might be useful for certain types of renderers.
    pub(crate) fn progress_completely<'a, 'bump, Dom: RealDom<'bump>>(
        &'bump self,
        diff_machine: &'_ mut DiffMachine<'a, 'bump, Dom>,
    ) -> Result<()> {
        // Add this component to the list of components that need to be difed
        // #[allow(unused_assignments)]
        // let mut cur_height: u32 = 0;

        // Now, there are events in the queue
        let mut updates = self.event_queue.0.as_ref().borrow_mut();

        // Order the nodes by their height, we want the nodes with the smallest depth on top
        // This prevents us from running the same component multiple times
        updates.sort_unstable();

        log::debug!("There are: {:#?} updates to be processed", updates.len());

        // Iterate through the triggered nodes (sorted by height) and begin to diff them
        for update in updates.drain(..) {
            log::debug!("Running updates for: {:#?}", update);
            // Make sure this isn't a node we've already seen, we don't want to double-render anything
            // If we double-renderer something, this would cause memory safety issues
            if diff_machine.seen_nodes.contains(&update.idx) {
                continue;
            }

            // Now, all the "seen nodes" are nodes that got notified by running this listener
            diff_machine.seen_nodes.insert(update.idx.clone());

            // Start a new mutable borrow to components
            // We are guaranteeed that this scope is unique because we are tracking which nodes have modified

            let cur_component = self.components.try_get_mut(update.idx).unwrap();
            // let inner: &'s mut _ = unsafe { &mut *self.components.0.borrow().arena.get() };
            // let cur_component = inner.get_mut(update.idx).unwrap();

            cur_component.run_scope()?;
            // diff_machine.change_list.load_known_root(1);

            let (old, new) = (cur_component.old_frame(), cur_component.next_frame());
            // let (old, new) = cur_component.get_frames_mut();
            diff_machine.diff_node(old, new);

            // cur_height = cur_component.height;

            // log::debug!(
            //     "Processing update: {:#?} with height {}",
            //     &update.idx,
            //     cur_height
            // );
        }

        Ok(())
    }

    pub fn base_scope(&self) -> &Scope {
        let idx = self.base_scope;
        self.components.try_get(idx).unwrap()
    }
}

// TODO!
// These impls are actually wrong. The DOM needs to have a mutex implemented.
unsafe impl Sync for VirtualDom {}
unsafe impl Send for VirtualDom {}
