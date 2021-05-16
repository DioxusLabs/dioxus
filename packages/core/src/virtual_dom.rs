//! # VirtualDOM Implementation for Rust
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
//! In this file, multiple items are defined. This file is big, but should be documented well to
//! navigate the innerworkings of the Dom. We try to keep these main mechanics in this file to limit
//! the possible exposed API surface (keep fields private). This particular implementation of VDOM
//! is extremely efficient, but relies on some unsafety under the hood to do things like manage
//! micro-heaps for components.
//!
//! Included is:
//! - The [`VirtualDom`] itself
//! - The [`Scope`] object for mangning component lifecycle
//! - The [`ActiveFrame`] object for managing the Scope`s microheap
//! - The [`Context`] object for exposing VirtualDOM API to components
//! - The [`NodeCtx`] object for lazyily exposing the `Context` API to the nodebuilder API
//! - The [`Hook`] object for exposing state management in components.
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.

use crate::innerlude::*;
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::TypeId,
    cell::{RefCell, UnsafeCell},
    collections::HashSet,
    fmt::Debug,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
};

pub use support::*;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    components: UnsafeCell<Arena<Scope>>,

    /// The index of the root component
    /// Should always be the first (gen0, id0)
    pub base_scope: ScopeIdx,

    /// All components dump their updates into a queue to be processed
    pub(crate) event_queue: EventQueue,

    /// a strong allocation to the "caller" for the original component and its props
    #[doc(hidden)]
    _root_caller: Rc<OpaqueComponent<'static>>,

    /// Type of the original props. This is stored as TypeId so VirtualDom does not need to be generic.
    ///
    /// Whenver props need to be updated, an Error will be thrown if the new props do not
    /// match the props used to create the VirtualDom.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,
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
    /// let dom = VirtualDom::new(|ctx, _| ctx.render(rsx!{ div {"hello world"} }));
    ///
    /// // or pass in...
    ///
    /// let root = |ctx, _| {
    ///     ctx.render(rsx!{
    ///         div {"hello world"}
    ///     })
    /// }
    /// let dom = VirtualDom::new(root);
    ///
    /// // or directly from a fn
    ///
    /// fn Example(ctx: Context, props: &()) -> DomTree  {
    ///     ctx.render(rsx!{ div{"hello world"} })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    /// ```ignore
    /// // Directly from a closure
    ///
    /// let dom = VirtualDom::new(|ctx, props| ctx.render(rsx!{ div {"hello world"} }));
    ///
    /// // or pass in...
    ///
    /// let root = |ctx, props| {
    ///     ctx.render(rsx!{
    ///         div {"hello world"}
    ///     })
    /// }
    /// let dom = VirtualDom::new(root);
    ///
    /// // or directly from a fn
    ///
    /// fn Example(ctx: Context, props: &SomeProps) -> DomTree  {
    ///     ctx.render(rsx!{ div{"hello world"} })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let mut components = Arena::new();

        // Normally, a component would be passed as a child in the RSX macro which automatically produces OpaqueComponents
        // Here, we need to make it manually, using an RC to force the Weak reference to stick around for the main scope.
        let _root_caller: Rc<OpaqueComponent> = Rc::new(move |ctx| root(ctx, &root_props));

        // Create a weak reference to the OpaqueComponent for the root scope to use as its render function
        let caller_ref = Rc::downgrade(&_root_caller);

        // Build a funnel for hooks to send their updates into. The `use_hook` method will call into the update funnel.
        let event_queue = EventQueue::default();
        let _event_queue = event_queue.clone();

        // Make the first scope
        // We don't run the component though, so renderers will need to call "rebuild" when they initialize their DOM
        let base_scope = components
            .insert_with(move |myidx| Scope::new(caller_ref, myidx, None, 0, _event_queue));

        Self {
            _root_caller,
            base_scope,
            event_queue,
            components: UnsafeCell::new(components),
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom. from scratch
    pub fn rebuild<'s>(&'s mut self) -> Result<EditList<'s>> {
        let mut diff_machine = DiffMachine::new();

        // Schedule an update and then immediately call it on the root component
        // This is akin to a hook being called from a listener and requring a re-render
        // Instead, this is done on top-level component
        unsafe {
            let components = &*self.components.get();
            let base = components.get(self.base_scope).unwrap();
            let update = self.event_queue.schedule_update(base);
            update();
        };

        self.progress_completely(&mut diff_machine)?;

        Ok(diff_machine.consume())
    }
}

// ======================================
// Private Methods for the VirtualDom
// ======================================
impl VirtualDom {
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
    // A good project would be to remove all unsafe from this crate and move the unsafety into abstractions.
    pub fn progress_with_event(&mut self, event: EventTrigger) -> Result<EditList> {
        let id = event.component_id.clone();

        unsafe {
            (&mut *self.components.get())
                .get_mut(id)
                .ok_or(Error::FatalInternal("Borrowing should not fail"))?
                .call_listener(event)?;
        }

        let mut diff_machine = DiffMachine::new();
        self.progress_completely(&mut diff_machine)?;

        Ok(diff_machine.consume())
    }

    pub(crate) fn progress_completely(&mut self, diff_machine: &mut DiffMachine) -> Result<()> {
        // Add this component to the list of components that need to be difed
        let mut cur_height: u32 = 0;

        // Now, there are events in the queue
        let mut seen_nodes = HashSet::<ScopeIdx>::new();
        let mut updates = self.event_queue.0.as_ref().borrow_mut();

        // Order the nodes by their height, we want the biggest nodes on the top
        // This prevents us from running the same component multiple times
        updates.sort_unstable();

        // Iterate through the triggered nodes (sorted by height) and begin to diff them
        for update in updates.drain(..) {
            // Make sure this isn't a node we've already seen, we don't want to double-render anything
            // If we double-renderer something, this would cause memory safety issues
            if seen_nodes.contains(&update.idx) {
                continue;
            }

            // Now, all the "seen nodes" are nodes that got notified by running this listener
            seen_nodes.insert(update.idx.clone());

            unsafe {
                // Start a new mutable borrow to components

                // We are guaranteeed that this scope is unique because we are tracking which nodes have modified
                let component = (&mut *self.components.get())
                    .get_mut(update.idx)
                    .expect("msg");

                component.run_scope()?;

                diff_machine.diff_node(component.old_frame(), component.next_frame());

                cur_height = component.height;
            }

            log::debug!(
                "Processing update: {:#?} with height {}",
                &update.idx,
                cur_height
            );

            // Now, the entire subtree has been invalidated. We need to descend depth-first and process
            // any updates that the diff machine has proprogated into the component lifecycle queue
            while let Some(event) = diff_machine.lifecycle_events.pop_front() {
                match event {
                    // A new component has been computed from the diffing algorithm
                    // create a new component in the arena, run it, move the diffing machine to this new spot, and then diff it
                    // this will flood the lifecycle queue with new updates to build up the subtree
                    LifeCycleEvent::Mount {
                        caller,
                        root_id: id,
                        stable_scope_addr,
                    } => {
                        log::debug!("Mounting a new component");

                        // We're modifying the component arena while holding onto references into the assoiated bump arenas of its children
                        // those references are stable, even if the component arena moves around in memory, thanks to the bump arenas.
                        // However, there is no way to convey this to rust, so we need to use unsafe to pierce through the lifetime.
                        let components: &mut _ = unsafe { &mut *self.components.get() };

                        // Insert a new scope into our component list
                        let idx = components.insert_with(|f| {
                            Scope::new(caller, f, None, cur_height + 1, self.event_queue.clone())
                        });

                        // Grab out that component
                        let component = components.get_mut(idx).unwrap();

                        // Actually initialize the caller's slot with the right address
                        *stable_scope_addr.upgrade().unwrap().as_ref().borrow_mut() = Some(idx);

                        // Run the scope for one iteration to initialize it
                        component.run_scope()?;

                        // Navigate the diff machine to the right point in the output dom
                        diff_machine.change_list.load_known_root(id);

                        // And then run the diff algorithm
                        diff_machine.diff_node(component.old_frame(), component.next_frame());

                        // Finally, insert this node as a seen node.
                        seen_nodes.insert(idx);
                    }

                    // A component has remained in the same location but its properties have changed
                    // We need to process this component and then dump the output lifecycle events into the queue
                    LifeCycleEvent::PropsChanged {
                        caller,
                        root_id,
                        stable_scope_addr,
                    } => {
                        log::debug!("Updating a component after its props have changed");

                        let components: &mut _ = unsafe { &mut *self.components.get() };

                        // Get the stable index to the target component
                        // This *should* exist due to guarantees in the diff algorithm
                        let idx = stable_scope_addr
                            .upgrade()
                            .unwrap()
                            .as_ref()
                            .borrow()
                            .unwrap();

                        // Grab out that component
                        let component = components.get_mut(idx).unwrap();

                        // We have to move the caller over or running the scope will fail
                        component.update_caller(caller);

                        // Run the scope
                        component.run_scope()?;

                        // Navigate the diff machine to the right point in the output dom
                        diff_machine.change_list.load_known_root(root_id);

                        // And then run the diff algorithm
                        diff_machine.diff_node(component.old_frame(), component.next_frame());

                        // Finally, insert this node as a seen node.
                        seen_nodes.insert(idx);
                    }

                    // A component's parent has updated, but its properties did not change.
                    // This means the caller ptr is invalidated and needs to be updated, but the component itself does not need to be re-ran
                    LifeCycleEvent::SameProps {
                        caller,
                        stable_scope_addr,
                        ..
                    } => {
                        // In this case, the parent made a new DomTree that resulted in the same props for us
                        // However, since our caller is located in a Bump frame, we need to update the caller pointer (which is now invalid)
                        log::debug!("Received the same props");

                        let components: &mut _ = unsafe { &mut *self.components.get() };

                        // Get the stable index to the target component
                        // This *should* exist due to guarantees in the diff algorithm
                        let idx = stable_scope_addr
                            .upgrade()
                            .unwrap()
                            .as_ref()
                            .borrow()
                            .unwrap();

                        // Grab out that component
                        let component = components.get_mut(idx).unwrap();

                        // We have to move the caller over or running the scope will fail
                        component.update_caller(caller);

                        // This time, we will not add it to our seen nodes since we did not actually run it
                    }

                    LifeCycleEvent::Remove {
                        root_id,
                        stable_scope_addr,
                    } => {
                        unimplemented!("This feature (Remove) is unimplemented")
                    }
                    LifeCycleEvent::Replace {
                        caller,
                        root_id: id,
                        ..
                    } => {
                        unimplemented!("This feature (Replace) is unimplemented")
                    }
                }
            }
        }

        Ok(())
    }
}

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // The parent's scope ID
    pub parent: Option<ScopeIdx>,

    // Our own ID accessible from the component map
    pub myidx: ScopeIdx,

    pub height: u32,

    pub event_queue: EventQueue,

    // A list of children
    // TODO, repalce the hashset with a faster hash function
    pub children: HashSet<ScopeIdx>,

    pub caller: Weak<OpaqueComponent<'static>>,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub(crate) frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // or we could dedicate a tiny bump arena just for them
    // could also use ourborous
    pub(crate) hooks: RefCell<Vec<Hook>>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    pub(crate) listeners: RefCell<Vec<*const dyn Fn(VirtualEvent)>>,
}

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    //
    // Scopes cannot be made anywhere else except for this file
    // Therefore, their lifetimes are connected exclusively to the virtual dom
    fn new<'creator_node>(
        caller: Weak<OpaqueComponent<'creator_node>>,
        myidx: ScopeIdx,
        parent: Option<ScopeIdx>,
        height: u32,
        event_queue: EventQueue,
    ) -> Self {
        log::debug!(
            "New scope created, height is {}, idx is {:?}",
            height,
            myidx
        );

        // The Componet has a lifetime that's "stuck" to its original allocation.
        // We need to "break" this reference and manually manage the lifetime.
        //
        // Not the best solution, so TODO on removing this in favor of a dedicated resource abstraction.
        let broken_caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent<'creator_node>>,
                Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        Self {
            caller: broken_caller,
            hooks: RefCell::new(Vec::new()),
            frames: ActiveFrame::new(),
            children: HashSet::new(),
            listeners: Default::default(),
            parent,
            myidx,
            height,
            event_queue,
        }
    }

    pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent<'creator_node>>) {
        let broken_caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent<'creator_node>>,
                Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        self.caller = broken_caller;
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'b>(&'b mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        self.frames.next().bump.reset();

        let ctx = Context {
            idx: 0.into(),
            _p: std::marker::PhantomData {},
            scope: self,
        };

        let caller = self
            .caller
            .upgrade()
            .ok_or(Error::FatalInternal("Failed to get caller"))?;

        let new_head = unsafe {
            // Cast the caller ptr from static to one with our own reference
            std::mem::transmute::<&OpaqueComponent<'static>, &OpaqueComponent<'b>>(caller.as_ref())
        }(ctx);

        self.frames.cur_frame_mut().head_node = new_head.root;

        Ok(())
    }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub fn call_listener(&mut self, trigger: EventTrigger) -> Result<()> {
        let EventTrigger {
            listener_id, event, ..
        } = trigger;

        unsafe {
            // Convert the raw ptr into an actual object
            // This operation is assumed to be safe
            let listener_fn = self
                .listeners
                .try_borrow()
                .ok()
                .ok_or(Error::FatalInternal("Borrowing listener failed "))?
                .get(listener_id as usize)
                .ok_or(Error::FatalInternal("Event should exist if triggered"))?
                .as_ref()
                .ok_or(Error::FatalInternal("Raw event ptr is invalid"))?;

            // Run the callback with the user event
            listener_fn(event);

            // drain all the event listeners
            // if we don't, then they'll stick around and become invalid
            // big big big big safety issue
            self.listeners
                .try_borrow_mut()
                .ok()
                .ok_or(Error::FatalInternal("Borrowing listener failed"))?
                .drain(..);
        }
        Ok(())
    }

    pub fn next_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    pub fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }

    pub fn cur_frame(&self) -> &BumpFrame {
        self.frames.cur_frame()
    }
}

pub struct ActiveFrame {
    // We use a "generation" for users of contents in the bump frames to ensure their data isn't broken
    pub generation: RefCell<usize>,

    // The double-buffering situation that we will use
    pub frames: [BumpFrame; 2],
}

pub struct BumpFrame {
    pub bump: Bump,
    pub head_node: VNode<'static>,
}

impl ActiveFrame {
    pub fn new() -> Self {
        Self::from_frames(
            BumpFrame {
                bump: Bump::new(),
                head_node: VNode::text(""),
            },
            BumpFrame {
                bump: Bump::new(),
                head_node: VNode::text(""),
            },
        )
    }

    fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            generation: 0.into(),
            frames: [a, b],
        }
    }

    fn cur_frame(&self) -> &BumpFrame {
        match *self.generation.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    fn cur_frame_mut(&mut self) -> &mut BumpFrame {
        match *self.generation.borrow() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    pub fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.generation.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };

        // Give out our self-referential item with our own borrowed lifetime
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    pub fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.generation.borrow() & 1 != 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };

        // Give out our self-referential item with our own borrowed lifetime
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    fn next(&mut self) -> &mut BumpFrame {
        *self.generation.borrow_mut() += 1;

        if *self.generation.borrow() % 2 == 0 {
            &mut self.frames[0]
        } else {
            &mut self.frames[1]
        }
    }
}

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
/// This lets components schedule updates, integrate hooks, and expose their context via the context api.
///
/// Properties passed down from the parent component are also directly accessible via the exposed "props" field.
///
/// ```ignore
/// #[derive(Properties)]
/// struct Props {
///     name: String
///
/// }
///
/// fn example(ctx: Context, props: &Props -> VNode {
///     html! {
///         <div> "Hello, {ctx.props.name}" </div>
///     }
/// }
/// ```
// todo: force lifetime of source into T as a valid lifetime too
// it's definitely possible, just needs some more messing around
pub struct Context<'src> {
    pub idx: RefCell<usize>,

    // pub scope: ScopeIdx,
    pub scope: &'src Scope,

    pub _p: std::marker::PhantomData<&'src ()>,
}

impl<'a> Context<'a> {
    /// Access the children elements passed into the component
    pub fn children(&self) -> Vec<VNode> {
        todo!("Children API not yet implemented for component Context")
    }

    pub fn callback(&self, _f: impl Fn(()) + 'a) {}

    /// Create a subscription that schedules a future render for the reference component
    pub fn schedule_update(&self) -> impl Fn() -> () {
        self.scope.event_queue.schedule_update(&self.scope)
    }

    /// Create a suspended component from a future.
    ///
    /// When the future completes, the component will be renderered
    pub fn suspend<F: for<'b> FnOnce(&'b NodeCtx<'a>) -> VNode<'a> + 'a>(
        &self,
        _fut: impl Future<Output = LazyNodes<'a, F>>,
    ) -> VNode<'a> {
        todo!()
    }
}

impl<'scope> Context<'scope> {
    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// This function consumes the context and absorb the lifetime, so these VNodes *must* be returned.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// fn Component(ctx: Context<Props>) -> VNode {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_tree = html! {<div> "Hello World" </div>};
    ///     
    ///     // Actually build the tree and allocate it
    ///     ctx.render(lazy_tree)
    /// }
    ///```
    pub fn render<F: for<'b> FnOnce(&'b NodeCtx<'scope>) -> VNode<'scope> + 'scope>(
        &self,
        lazy_nodes: LazyNodes<'scope, F>,
    ) -> DomTree {
        let ctx = NodeCtx {
            scope_ref: self.scope,
            idx: 0.into(),
        };

        let safe_nodes: VNode<'scope> = lazy_nodes.into_vnode(&ctx);
        let root: VNode<'static> = unsafe { std::mem::transmute(safe_nodes) };
        DomTree { root }
    }
}

type Hook = Pin<Box<dyn std::any::Any>>;

impl<'scope> Context<'scope> {
    /// Store a value between renders
    ///
    /// - Initializer: closure used to create the initial hook state
    /// - Runner: closure used to output a value every time the hook is used
    /// - Cleanup: closure used to teardown the hook once the dom is cleaned up
    ///
    /// ```ignore
    /// // use_ref is the simplest way of storing a value between renders
    /// pub fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T + 'static) -> Rc<RefCell<T>> {
    ///     use_hook(
    ///         || Rc::new(RefCell::new(initial_value())),
    ///         |state, _| state.clone(),
    ///         |_| {},
    ///     )
    /// }
    /// ```
    pub fn use_hook<'c, InternalHookState: 'static, Output: 'scope>(
        &'c self,

        // The closure that builds the hook state
        initializer: impl FnOnce() -> InternalHookState,

        // The closure that takes the hookstate and returns some value
        runner: impl FnOnce(&'scope mut InternalHookState) -> Output,

        // The closure that cleans up whatever mess is left when the component gets torn down
        // TODO: add this to the "clean up" group for when the component is dropped
        _cleanup: impl FnOnce(InternalHookState),
    ) -> Output {
        let idx = *self.idx.borrow();

        // Grab out the hook list
        let mut hooks = self.scope.hooks.borrow_mut();

        // If the idx is the same as the hook length, then we need to add the current hook
        if idx >= hooks.len() {
            let new_state = initializer();
            hooks.push(Box::pin(new_state));
        }

        *self.idx.borrow_mut() += 1;

        let stable_ref = hooks
            .get_mut(idx)
            .expect("Should not fail, idx is validated")
            .as_mut();

        let pinned_state = unsafe { Pin::get_unchecked_mut(stable_ref) };

        let internal_state = pinned_state.downcast_mut::<InternalHookState>().expect(
            r###"
                Unable to retrive the hook that was initialized in this index.
                Consult the `rules of hooks` to understand how to use hooks properly.

                You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
            "###,
        );

        // We extend the lifetime of the internal state
        runner(unsafe { &mut *(internal_state as *mut _) })
    }
}

mod support {
    use super::*;

    // We actually allocate the properties for components in their parent's properties
    // We then expose a handle to use those props for render in the form of "OpaqueComponent"
    pub(crate) type OpaqueComponent<'a> = dyn for<'b> Fn(Context<'b>) -> DomTree + 'a;

    #[derive(Debug, Default, Clone)]
    pub struct EventQueue(pub(crate) Rc<RefCell<Vec<HeightMarker>>>);

    impl EventQueue {
        pub fn schedule_update(&self, source: &Scope) -> impl Fn() {
            let inner = self.clone();
            let marker = HeightMarker {
                height: source.height,
                idx: source.myidx,
            };
            move || inner.0.as_ref().borrow_mut().push(marker)
        }
    }

    /// A helper type that lets scopes be ordered by their height
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct HeightMarker {
        pub idx: ScopeIdx,
        pub height: u32,
    }

    impl Ord for HeightMarker {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.height.cmp(&other.height)
        }
    }

    impl PartialOrd for HeightMarker {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    // NodeCtx is used to build VNodes in the component's memory space.
    // This struct adds metadata to the final DomTree about listeners, attributes, and children
    #[derive(Clone)]
    pub struct NodeCtx<'a> {
        pub scope_ref: &'a Scope,
        pub idx: RefCell<usize>,
    }

    impl<'a> NodeCtx<'a> {
        pub fn bump(&self) -> &'a Bump {
            &self.scope_ref.cur_frame().bump
        }
    }

    impl Debug for NodeCtx<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Ok(())
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn simulate() {
        let dom = VirtualDom::new(|ctx, props| {
            //
            ctx.render(rsx! {
                div {

                }
            })
        });
        // let root = dom.components.get(dom.base_scope).unwrap();
    }
}
