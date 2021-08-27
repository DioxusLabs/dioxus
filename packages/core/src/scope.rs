use crate::innerlude::*;
use bumpalo::boxed::Box as BumpBox;
use futures_channel::mpsc::UnboundedSender;
use fxhash::FxHashSet;
use std::{
    any::{Any, TypeId},
    borrow::BorrowMut,
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
    pub(crate) descendents: RefCell<FxHashSet<ScopeId>>,

    // Nodes
    // an internal, highly efficient storage of vnodes
    // lots of safety condsiderations
    pub(crate) frames: ActiveFrame,
    pub(crate) caller: Rc<WrappedCaller>,
    pub(crate) child_nodes: ScopeChildren<'static>,
    pub(crate) pending_garbage: RefCell<Vec<*const VNode<'static>>>,

    // Listeners
    pub(crate) listeners: RefCell<Vec<*const Listener<'static>>>,
    pub(crate) borrowed_props: RefCell<Vec<*const VComponent<'static>>>,

    pub(crate) suspended_nodes: RefCell<HashMap<u64, *const VNode<'static>>>,

    // State
    pub(crate) hooks: HookList,
    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    pub(crate) memoized_updater: Rc<dyn Fn() + 'static>,

    pub(crate) shared: EventChannel,
}

// The type of closure that wraps calling components
pub type WrappedCaller = dyn for<'b> Fn(&'b Scope) -> DomTree<'b>;

/// The type of task that gets sent to the task scheduler
/// Submitting a fiber task returns a handle to that task, which can be used to wake up suspended nodes
pub type FiberTask = Pin<Box<dyn Future<Output = ScopeId>>>;

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

        child_nodes: ScopeChildren,

        shared: EventChannel,
    ) -> Self {
        let child_nodes = unsafe { child_nodes.extend_lifetime() };

        let up = shared.schedule_any_immediate.clone();
        let memoized_updater = Rc::new(move || up(arena_idx));

        Self {
            memoized_updater,
            shared,
            child_nodes,
            caller,
            parent_idx: parent,
            our_arena_idx: arena_idx,
            height,
            frames: ActiveFrame::new(),
            hooks: Default::default(),
            suspended_nodes: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            borrowed_props: Default::default(),
            descendents: Default::default(),
            pending_garbage: Default::default(),
        }
    }

    pub(crate) fn update_scope_dependencies<'creator_node>(
        &mut self,
        caller: Rc<WrappedCaller>,
        child_nodes: ScopeChildren,
    ) {
        self.caller = caller;
        let child_nodes = unsafe { child_nodes.extend_lifetime() };
        self.child_nodes = child_nodes;
    }

    pub(crate) fn run_scope<'sel>(&'sel mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners

        self.ensure_drop_safety();

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        unsafe { self.hooks.reset() };

        // Safety:
        // - We've dropped all references to the wip bump frame
        unsafe { self.frames.reset_wip_frame() };

        // Cast the caller ptr from static to one with our own reference
        let render: &WrappedCaller = self.caller.as_ref();

        match render(self) {
            None => {
                // the user's component failed. We avoid cycling to the next frame
                log::error!("Running your component failed! It will no longer receive events.");
                Err(Error::ComponentFailed)
            }
            Some(new_head) => {
                // the user's component succeeded. We can safely cycle to the next frame
                self.frames.wip_frame_mut().head_node = unsafe { std::mem::transmute(new_head) };
                self.frames.cycle_frame();
                log::debug!("Successfully rendered component");
                Ok(())
            }
        }
    }

    /// This method cleans up any references to data held within our hook list. This prevents mutable aliasing from
    /// causuing UB in our tree.
    ///
    /// This works by cleaning up our references from the bottom of the tree to the top. The directed graph of components
    /// essentially forms a dependency tree that we can traverse from the bottom to the top. As we traverse, we remove
    /// any possible references to the data in the hook list.
    ///
    /// Refrences to hook data can only be stored in listeners and component props. During diffing, we make sure to log
    /// all listeners and borrowed props so we can clear them here.
    fn ensure_drop_safety(&mut self) {
        // make sure all garabge is collected before trying to proceed with anything else
        debug_assert!(
            self.pending_garbage.borrow().is_empty(),
            "clean up your garabge please"
        );

        // todo!("arch changes");

        // // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
        // // run the hooks (which hold an &mut Referrence)
        // // right now, we don't drop
        // // let vdom = &self.vdom;
        // self.borrowed_props
        //     .get_mut()
        //     .drain(..)
        //     .map(|li| unsafe { &*li })
        //     .for_each(|comp| {
        //         // First drop the component's undropped references
        //         let scope_id = comp.ass_scope.get().unwrap();
        //         let scope = unsafe { vdom.get_scope_mut(scope_id) }.unwrap();
        //         scope.ensure_drop_safety();

        //         // Now, drop our own reference
        //         let mut dropper = comp.drop_props.borrow_mut().take().unwrap();
        //         dropper();
        //     });

        // // Now that all the references are gone, we can safely drop our own references in our listeners.
        // self.listeners
        //     .get_mut()
        //     .drain(..)
        //     .map(|li| unsafe { &*li })
        //     .for_each(|listener| {
        //         listener.callback.borrow_mut().take();
        //     });
    }

    // A safe wrapper around calling listeners
    //
    //
    pub(crate) fn call_listener(
        &mut self,
        event: SyntheticEvent,
        element: ElementId,
    ) -> Result<()> {
        let listners = self.listeners.borrow_mut();

        let raw_listener = listners.iter().find(|lis| {
            let search = unsafe { &***lis };
            let search_id = search.mounted_node.get();

            // this assumes the node might not be mounted - should we assume that though?
            match search_id.map(|f| f == element) {
                Some(same) => same,
                None => false,
            }
        });

        if let Some(raw_listener) = raw_listener {
            let listener = unsafe { &**raw_listener };
            let mut cb = listener.callback.borrow_mut();
            if let Some(cb) = cb.as_mut() {
                (cb)(event);
            }
        } else {
            log::warn!("An event was triggered but there was no listener to handle it");
        }

        Ok(())
    }

    pub fn root(&self) -> &VNode {
        self.frames.fin_head()
    }

    pub fn child_nodes<'a>(&'a self) -> ScopeChildren {
        unsafe { self.child_nodes.unextend_lfetime() }
    }

    pub fn consume_garbage(&self) -> Vec<&VNode> {
        self.pending_garbage
            .borrow_mut()
            .drain(..)
            .map(|node| {
                // safety: scopes cannot cycle without their garbage being collected. these nodes are safe
                let node: &VNode<'static> = unsafe { &*node };
                let node: &VNode = unsafe { std::mem::transmute(node) };
                node
            })
            .collect::<Vec<_>>()
    }
}
