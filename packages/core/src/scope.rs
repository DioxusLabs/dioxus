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

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // The parent's scope ID
    pub parent: Option<ScopeIdx>,

    // IDs of children that this scope has created
    // This enables us to drop the children and their children when this scope is destroyed
    pub(crate) descendents: RefCell<HashSet<ScopeIdx>>,

    pub child_nodes: &'static [VNode<'static>],

    // A reference to the list of components.
    // This lets us traverse the component list whenever we need to access our parent or children.
    pub arena_link: ScopeArena,

    pub shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // Our own ID accessible from the component map
    pub arena_idx: ScopeIdx,

    pub height: u32,

    pub event_channel: Rc<dyn Fn() + 'static>,

    pub caller: Weak<OpaqueComponent>,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // or we could dedicate a tiny bump arena just for them
    // could also use ourborous
    pub hooks: HookList,

    pub(crate) listener_idx: Cell<usize>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    pub(crate) listeners: RefCell<Vec<(*mut Cell<RealDomNode>, *mut dyn FnMut(VirtualEvent))>>,

    pub(crate) suspended_tasks: Vec<*mut Pin<Box<dyn Future<Output = VNode<'static>>>>>,
}

// We need to pin the hook so it doesn't move as we initialize the list of hooks
type Hook = Box<dyn std::any::Any>;
type EventChannel = Rc<dyn Fn()>;

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    //
    // Scopes cannot be made anywhere else except for this file
    // Therefore, their lifetimes are connected exclusively to the virtual dom
    pub fn new<'creator_node>(
        caller: Weak<OpaqueComponent>,
        arena_idx: ScopeIdx,
        parent: Option<ScopeIdx>,
        height: u32,
        event_channel: EventChannel,
        arena_link: ScopeArena,
        child_nodes: &'creator_node [VNode<'creator_node>],
    ) -> Self {
        log::debug!(
            "New scope created, height is {}, idx is {:?}",
            height,
            arena_idx
        );

        // The function to run this scope is actually located in the parent's bump arena.
        // Every time the parent is updated, that function is invalidated via double-buffering wiping the old frame.
        // If children try to run this invalid caller, it *will* result in UB.
        //
        // During the lifecycle progression process, this caller will need to be updated. Right now,
        // until formal safety abstractions are implemented, we will just use unsafe to "detach" the caller
        // lifetime from the bump arena, exposing ourselves to this potential for invalidation. Truthfully,
        // this is a bit of a hack, but will remain this way until we've figured out a cleaner solution.
        //
        // Not the best solution, so TODO on removing this in favor of a dedicated resource abstraction.
        let caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent>,
                Weak<OpaqueComponent>,
                // Weak<OpaqueComponent<'creator_node>>,
                // Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        let child_nodes = unsafe { std::mem::transmute(child_nodes) };

        Self {
            child_nodes,
            caller,
            parent,
            arena_idx,
            height,
            event_channel,
            arena_link,
            listener_idx: Default::default(),
            frames: ActiveFrame::new(),
            hooks: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            descendents: Default::default(),
            suspended_tasks: Default::default(),
        }
    }

    pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent>) {
        // pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent<'creator_node>>) {
        let broken_caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent>,
                Weak<OpaqueComponent>,
                // Weak<OpaqueComponent<'creator_node>>,
                // Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        self.caller = broken_caller;
    }

    pub fn update_children<'creator_node>(
        &mut self,
        child_nodes: &'creator_node [VNode<'creator_node>],
    ) {
        let child_nodes = unsafe { std::mem::transmute(child_nodes) };
        self.child_nodes = child_nodes;
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'sel>(&'sel mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        self.frames.next().bump.reset();

        // Remove all the outdated listeners
        self.listeners.borrow_mut().clear();

        unsafe { self.hooks.reset() };
        self.listener_idx.set(0);

        let caller = self
            .caller
            .upgrade()
            .ok_or(Error::FatalInternal("Failed to get caller"))?;

        // Cast the caller ptr from static to one with our own reference
        let c2: &OpaqueComponent = caller.as_ref();
        let c3: &OpaqueComponent = unsafe { std::mem::transmute(c2) };

        self.frames.cur_frame_mut().head_node = unsafe { self.own_vnodes(c3) };

        Ok(())
    }

    // this is its own function so we can preciesly control how lifetimes flow
    unsafe fn own_vnodes<'a>(&'a self, f: &OpaqueComponent) -> VNode<'static> {
        let new_head: VNode<'a> = f(self);
        let out: VNode<'static> = std::mem::transmute(new_head);
        out
    }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub fn call_listener(&mut self, trigger: EventTrigger) -> Result<()> {
        let EventTrigger {
            real_node_id,
            event,
            ..
        } = trigger;

        // todo: implement scanning for outdated events

        // Convert the raw ptr into an actual object
        // This operation is assumed to be safe

        log::debug!("Calling listeners! {:?}", self.listeners.borrow().len());
        let mut listners = self.listeners.borrow_mut();
        let (_, listener) = listners
            .iter()
            .find(|(domptr, _)| {
                let p = unsafe { &**domptr };
                p.get() == real_node_id
            })
            .expect(&format!(
                "Failed to find real node with ID {:?}",
                real_node_id
            ));

        // TODO: Don'tdo a linear scan! Do a hashmap lookup! It'll be faster!
        unsafe {
            let mut listener_fn = &mut **listener;
            listener_fn(event);
        }

        Ok(())
    }

    pub(crate) fn next_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    pub(crate) fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }

    pub(crate) fn cur_frame(&self) -> &BumpFrame {
        self.frames.cur_frame()
    }

    pub(crate) fn root<'a>(&'a self) -> &'a VNode<'a> {
        &self.frames.cur_frame().head_node
    }
}
