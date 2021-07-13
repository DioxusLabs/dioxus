use crate::hooklist::HookList;
use crate::{arena::SharedArena, innerlude::*};

use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    rc::Rc,
};

// We need to pin the hook so it doesn't move as we initialize the list of hooks
type Hook = Box<dyn std::any::Any>;
type EventChannel = Rc<dyn Fn()>;
pub type WrappedCaller = dyn for<'b> Fn(&'b Scope) -> VNode<'b>;

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
    pub arena_link: SharedArena,

    pub shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // Our own ID accessible from the component map
    pub arena_idx: ScopeIdx,

    pub height: u32,

    pub event_channel: Rc<dyn Fn() + 'static>,

    pub caller: Rc<WrappedCaller>,

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

    pub task_submitter: TaskSubmitter,

    pub(crate) suspended_tasks: Vec<*mut Pin<Box<dyn Future<Output = VNode<'static>>>>>,
}

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
        arena_idx: ScopeIdx,
        parent: Option<ScopeIdx>,
        height: u32,
        event_channel: EventChannel,
        arena_link: SharedArena,
        child_nodes: &'creator_node [VNode<'creator_node>],
        task_submitter: TaskSubmitter,
    ) -> Self {
        log::debug!(
            "New scope created, height is {}, idx is {:?}",
            height,
            arena_idx
        );

        let child_nodes = unsafe { std::mem::transmute(child_nodes) };

        Self {
            child_nodes,
            caller,
            parent,
            arena_idx,
            height,
            event_channel,
            arena_link,
            task_submitter,
            listener_idx: Default::default(),
            frames: ActiveFrame::new(),
            hooks: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            descendents: Default::default(),
            suspended_tasks: Default::default(),
        }
    }

    pub fn update_caller<'creator_node>(&mut self, caller: Rc<WrappedCaller>) {
        self.caller = caller;
    }

    pub fn update_children<'creator_node>(
        &mut self,
        child_nodes: &'creator_node [VNode<'creator_node>],
    ) {
        let child_nodes = unsafe { std::mem::transmute(child_nodes) };
        self.child_nodes = child_nodes;
    }

    pub fn run_scope<'sel>(&'sel mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        self.frames.next().bump.reset();

        log::debug!("clearing listeners!");
        // Remove all the outdated listeners
        self.listeners.borrow_mut().clear();

        unsafe { self.hooks.reset() };
        self.listener_idx.set(0);

        // Cast the caller ptr from static to one with our own reference
        let c3: &WrappedCaller = self.caller.as_ref();

        self.frames.cur_frame_mut().head_node = unsafe { self.call_user_component(c3) };

        Ok(())
    }

    // this is its own function so we can preciesly control how lifetimes flow
    unsafe fn call_user_component<'a>(&'a self, caller: &WrappedCaller) -> VNode<'static> {
        let new_head: VNode<'a> = caller(self);
        std::mem::transmute(new_head)
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

        log::debug!(
            "There are  {:?} listeners associated with this scope {:#?}",
            self.listeners.borrow().len(),
            self.arena_idx
        );

        let listners = self.listeners.borrow_mut();

        // let listener = listners.get(trigger);
        let raw_listener = listners.iter().find(|(domptr, _)| {
            let search = unsafe { &**domptr };
            let search_id = search.get();
            log::info!("searching listener {:#?}", search_id);
            match real_node_id {
                Some(e) => search_id == e,
                None => false,
            }
        });

        match raw_listener {
            Some((_node, listener)) => unsafe {
                // TODO: Don'tdo a linear scan! Do a hashmap lookup! It'll be faster!
                let listener_fn = &mut **listener;
                listener_fn(event);
            },
            None => todo!(),
        }

        Ok(())
    }

    pub fn submit_task(&self, task: &mut Pin<Box<dyn Future<Output = ()>>>) {
        log::debug!("Task submitted into scope");
        (self.task_submitter)(DTask::new(task, self.arena_idx));
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

    #[inline]
    pub fn root<'a>(&'a self) -> &'a VNode<'a> {
        &self.frames.cur_frame().head_node
    }
}
