use crate::{error::Error, innerlude::*};
use crate::{innerlude::hooks::Hook, patch::Edit};
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, UnsafeCell},
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet},
    rc::{Rc, Weak},
};
use thiserror::private::AsDynError;

// We actually allocate the properties for components in their parent's properties
// We then expose a handle to use those props for render in the form of "OpaqueComponent"
pub(crate) type OpaqueComponent<'a> = dyn for<'b> Fn(Context<'b>) -> DomTree + 'a;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    components: UnsafeCell<Arena<Scope>>,

    /// The index of the root component.\
    /// Should always be the first
    pub base_scope: ScopeIdx,

    /// All components dump their updates into a queue to be processed
    pub(crate) update_schedule: UpdateFunnel,

    // a strong allocation to the "caller" for the original props
    #[doc(hidden)]
    root_caller: Rc<OpaqueComponent<'static>>,

    // Type of the original props. This is done so VirtualDom does not need to be generic.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,
}

// ======================================
// Public Methods for the VirtualDOM
// ======================================
impl VirtualDom {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let mut components = Arena::new();

        // The user passes in a "root" component (IE the function)
        // When components are used in the rsx! syntax, the parent assumes ownership
        // Here, the virtual dom needs to own the function, wrapping it with a `context caller`
        // The RC holds this component with a hard allocation
        let root_caller: Rc<OpaqueComponent> = Rc::new(move |ctx| root(ctx, &root_props));

        // To make it easier to pass the root around, we just leak it
        // When the virtualdom is dropped, we unleak it, so that unsafe isn't here, but it's important to remember
        let leaked_caller = Rc::downgrade(&root_caller);

        Self {
            root_caller,
            base_scope: components
                .insert_with(move |myidx| Scope::new(leaked_caller, myidx, None, 0)),
            components: UnsafeCell::new(components),
            update_schedule: UpdateFunnel::default(),
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom. from scratch
    pub fn rebuild<'s>(&'s mut self) -> Result<EditList<'s>> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeightMarker {
    idx: ScopeIdx,
    height: u32,
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
    pub fn progress_with_event(&mut self, event: EventTrigger) -> Result<EditList> {
        let id = event.component_id.clone();

        unsafe {
            (&mut *self.components.get())
                .get_mut(id)
                .expect("Borrowing should not fail")
                .call_listener(event)?;
        }

        // Add this component to the list of components that need to be difed
        let mut diff_machine = DiffMachine::new();
        let mut cur_height = 0;

        // Now, there are events in the queue
        let mut seen_nodes = HashSet::<ScopeIdx>::new();
        let mut updates = self.update_schedule.0.as_ref().borrow_mut();

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

                cur_height = component.height + 1;
            }

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
                        let idx =
                            components.insert_with(|f| Scope::new(caller, f, None, cur_height));

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
                        root_id,
                        stable_scope_addr,
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

        Ok(diff_machine.consume())
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // Drop all the components first
        // self.components.drain();

        // Finally, drop the root caller
        unsafe {
            // let root: Box<OpaqueComponent<'static>> =
            //     Box::from_raw(self.root_caller as *const OpaqueComponent<'static> as *mut _);

            // std::mem::drop(root);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct UpdateFunnel(Rc<RefCell<Vec<HeightMarker>>>);

impl UpdateFunnel {
    fn schedule_update(&self, source: &Scope) -> impl Fn() {
        let inner = self.clone();
        let marker = HeightMarker {
            height: source.height,
            idx: source.myidx,
        };
        move || inner.0.as_ref().borrow_mut().push(marker)
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

    // A list of children
    // TODO, repalce the hashset with a faster hash function
    pub children: HashSet<ScopeIdx>,

    // caller: &'static OpaqueComponent<'static>,
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
    ) -> Self {
        log::debug!(
            "New scope created, height is {}, idx is {:?}",
            height,
            myidx
        );
        // Caller has been broken free
        // However, it's still weak, so if the original Rc gets killed, we can't touch it
        let broken_caller: Weak<OpaqueComponent<'static>> = unsafe { std::mem::transmute(caller) };

        Self {
            caller: broken_caller,
            hooks: RefCell::new(Vec::new()),
            frames: ActiveFrame::new(),
            children: HashSet::new(),
            listeners: Default::default(),
            parent,
            myidx,
            height,
        }
    }
    pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent<'creator_node>>) {
        let broken_caller: Weak<OpaqueComponent<'static>> = unsafe { std::mem::transmute(caller) };

        self.caller = broken_caller;
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'b>(&'b mut self) -> Result<()> {
        // cycle to the next frame and then reset it
        // this breaks any latent references
        self.frames.next().bump.reset();

        let ctx = Context {
            idx: 0.into(),
            _p: std::marker::PhantomData {},
            scope: self,
        };

        let caller = self.caller.upgrade().expect("Failed to get caller");

        /*
        SAFETY ALERT

        DO NOT USE THIS VNODE WITHOUT THE APPOPRIATE ACCESSORS.
        KEEPING THIS STATIC REFERENCE CAN LEAD TO UB.

        Some things to note:
        - The VNode itself is bound to the lifetime, but it itself is owned by scope.
        - The VNode has a private API and can only be used from accessors.
        - Public API cannot drop or destructure VNode
        */
        let new_head = unsafe {
            // use the same type, just manipulate the lifetime
            type ComComp<'c> = Rc<OpaqueComponent<'c>>;
            let caller = std::mem::transmute::<ComComp<'static>, ComComp<'b>>(caller);
            (caller.as_ref())(ctx)
        };

        self.frames.cur_frame_mut().head_node = new_head.root;
        Ok(())
    }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub fn call_listener(&mut self, trigger: EventTrigger) -> Result<()> {
        let EventTrigger {
            listener_id,
            event: source,
            ..
        } = trigger;

        unsafe {
            // Convert the raw ptr into an actual object
            // This operation is assumed to be safe

            log::debug!("Running listener");

            self.listeners
                .borrow()
                .get(listener_id as usize)
                .ok_or(Error::FatalInternal("Event should exist if it was triggered"))?
                .as_ref()
                .ok_or(Error::FatalInternal("Raw event ptr is invalid"))?
                // Run the callback with the user event
                (source);

            log::debug!("Listener finished");

            // drain all the event listeners
            // if we don't, then they'll stick around and become invalid
            // big big big big safety issue
            self.listeners.borrow_mut().drain(..);
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

// ==========================
// Active-frame related code
// ==========================
// todo, do better with the active frame stuff
// somehow build this vnode with a lifetime tied to self
// This root node has  "static" lifetime, but it's really not static.
// It's goverened by the oldest of the two frames and is switched every time a new render occurs
// Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
// ! do not copy this reference are things WILL break !
pub struct ActiveFrame {
    pub idx: RefCell<usize>,
    pub frames: [BumpFrame; 2],

    // We use a "generation" for users of contents in the bump frames to ensure their data isn't broken
    pub generation: u32,
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
            idx: 0.into(),
            frames: [a, b],
            generation: 0,
        }
    }

    fn cur_frame(&self) -> &BumpFrame {
        match *self.idx.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    fn cur_frame_mut(&mut self) -> &mut BumpFrame {
        match *self.idx.borrow() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    pub fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.idx.borrow() & 1 == 0 {
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
        let raw_node = match *self.idx.borrow() & 1 != 0 {
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
        *self.idx.borrow_mut() += 1;

        if *self.idx.borrow() % 2 == 0 {
            &mut self.frames[0]
        } else {
            &mut self.frames[1]
        }
    }
}

mod test {
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

    // // This marker is designed to ensure resources shared from one bump to another are handled properly
    // // The underlying T may be already freed if there is an issue with our crate
    // pub(crate) struct BumpResource<T: 'static> {
    //     resource: T,
    //     scope: ScopeIdx,
    //     gen: u32,
    // }
}
