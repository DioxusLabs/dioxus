use crate::innerlude::*;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    rc::Rc,
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
///
/// We expose the `Scope` type so downstream users can traverse the Dioxus VirtualDOM for whatever
/// usecase they might have.
pub struct Scope {
    // Book-keeping about our spot in the arena
    pub(crate) parent_idx: Option<ScopeId>,
    pub(crate) our_arena_idx: ScopeId,
    pub(crate) height: u32,
    pub(crate) descendents: RefCell<HashSet<ScopeId>>,

    // Nodes
    // an internal, highly efficient storage of vnodes
    pub(crate) frames: ActiveFrame,
    pub(crate) caller: Rc<WrappedCaller>,
    pub(crate) child_nodes: &'static [VNode<'static>],

    // Listeners
    pub(crate) listeners: RefCell<Vec<*mut Listener<'static>>>,
    pub(crate) listener_idx: Cell<usize>,

    // State
    pub(crate) hooks: HookList,
    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // A reference to the resources shared by all the comonents
    pub(crate) vdom: SharedResources,
}

// The type of closure that wraps calling components
pub type WrappedCaller = dyn for<'b> Fn(&'b Scope) -> DomTree<'b>;

// The type of task that gets sent to the task scheduler
pub type FiberTask = Pin<Box<dyn Future<Output = EventTrigger>>>;

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    //
    // Scopes cannot be made anywhere else except for this file
    // Therefore, their lifetimes are connected exclusively to the virtual dom
    pub fn new<'creator_node>(
        caller: Rc<WrappedCaller>,

        arena_idx: ScopeId,

        parent: Option<ScopeId>,

        height: u32,

        child_nodes: &'creator_node [VNode<'creator_node>],

        vdom: SharedResources,
    ) -> Self {
        let child_nodes = unsafe { std::mem::transmute(child_nodes) };
        Self {
            child_nodes,
            caller,
            parent_idx: parent,
            our_arena_idx: arena_idx,
            height,
            vdom,
            listener_idx: Default::default(),
            frames: ActiveFrame::new(),
            hooks: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            descendents: Default::default(),
        }
    }

    pub(crate) fn update_caller<'creator_node>(&mut self, caller: Rc<WrappedCaller>) {
        self.caller = caller;
    }

    pub(crate) fn update_children<'creator_node>(
        &mut self,
        child_nodes: &'creator_node [VNode<'creator_node>],
    ) {
        let child_nodes = unsafe { std::mem::transmute(child_nodes) };
        self.child_nodes = child_nodes;
    }

    pub(crate) fn run_scope<'sel>(&'sel mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners

        // This is a very dangerous operation
        let next_frame = self.frames.prev_frame_mut();
        next_frame.bump.reset();

        self.listeners.borrow_mut().clear();

        unsafe { self.hooks.reset() };
        self.listener_idx.set(0);

        // Cast the caller ptr from static to one with our own reference
        let c3: &WrappedCaller = self.caller.as_ref();

        match c3(self) {
            None => {
                // the user's component failed. We avoid cycling to the next frame
                log::error!("Running your component failed! It will no longer receive events.");
                Err(Error::ComponentFailed)
            }
            Some(new_head) => {
                // the user's component succeeded. We can safely cycle to the next frame
                self.frames.prev_frame_mut().head_node = unsafe { std::mem::transmute(new_head) };
                self.frames.cycle_frame();
                Ok(())
            }
        }
    }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub(crate) fn call_listener(&mut self, trigger: EventTrigger) -> Result<()> {
        let EventTrigger {
            real_node_id,
            event,
            ..
        } = trigger;

        if let &VirtualEvent::AsyncEvent { .. } = &event {
            log::info!("arrived a fiber event");
            return Ok(());
        }

        log::debug!(
            "There are  {:?} listeners associated with this scope {:#?}",
            self.listeners.borrow().len(),
            self.our_arena_idx
        );

        let listners = self.listeners.borrow_mut();

        let raw_listener = listners.iter().find(|lis| {
            let search = unsafe { &mut ***lis };
            let search_id = search.mounted_node.get();
            log::info!("searching listener {:#?}", search_id);

            match (real_node_id, search_id) {
                (Some(e), Some(search_id)) => search_id == e,
                _ => false,
            }
        });

        if let Some(raw_listener) = raw_listener {
            let listener = unsafe { &mut **raw_listener };
            (listener.callback)(event);
        } else {
            log::warn!("An event was triggered but there was no listener to handle it");
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn next_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    #[inline]
    pub(crate) fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }

    #[inline]
    pub(crate) fn cur_frame(&self) -> &BumpFrame {
        self.frames.cur_frame()
    }

    /// Get the root VNode of this component
    #[inline]
    pub fn root<'a>(&'a self) -> &'a VNode<'a> {
        &self.frames.current_head_node()
    }
    #[inline]
    pub fn child_nodes<'a>(&'a self) -> &'a [VNode<'a>] {
        unsafe { std::mem::transmute(self.child_nodes) }
    }
}
