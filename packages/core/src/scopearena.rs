use slab::Slab;
use std::{
    borrow::BorrowMut,
    cell::{Cell, RefCell},
};

use bumpalo::{boxed::Box as BumpBox, Bump};
use futures_channel::mpsc::UnboundedSender;

use crate::innerlude::*;

pub type FcSlot = *const ();
// pub heuristics: FxHashMap<FcSlot, Heuristic>,

pub struct Heuristic {
    hook_arena_size: usize,
    node_arena_size: usize,
}

// a slab-like arena with stable references even when new scopes are allocated
// uses a bump arena as a backing
//
// has an internal heuristics engine to pre-allocate arenas to the right size
pub(crate) struct ScopeArena {
    bump: Bump,
    scopes: Vec<*mut Scope>,
    free_scopes: Vec<ScopeId>,
    nodes: RefCell<Slab<*const VNode<'static>>>,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
}

impl ScopeArena {
    pub fn new(sender: UnboundedSender<SchedulerMsg>) -> Self {
        Self {
            bump: Bump::new(),
            scopes: Vec::new(),
            free_scopes: Vec::new(),
            nodes: RefCell::new(Slab::new()),
            sender,
        }
    }

    pub fn get_scope(&self, id: &ScopeId) -> Option<&Scope> {
        unsafe { Some(&*self.scopes[id.0]) }
    }

    // this is unsafe
    pub unsafe fn get_scope_mut(&self, id: &ScopeId) -> Option<*mut Scope> {
        Some(self.scopes[id.0])
    }

    pub fn new_with_key(
        &mut self,
        fc_ptr: *const (),
        caller: *const dyn Fn(&Scope) -> Element,
        parent_scope: Option<*mut Scope>,
        height: u32,
        subtree: u32,
    ) -> ScopeId {
        if let Some(id) = self.free_scopes.pop() {
            // have already called drop on it - the slot is still chillin tho
            let scope = unsafe { &mut *self.scopes[id.0 as usize] };

            todo!("override the scope contents");
            id
        } else {
            let scope_id = ScopeId(self.scopes.len());

            let old_root = NodeLink {
                link_idx: 0,
                gen_id: 0,
                scope_id,
            };
            let new_root = NodeLink {
                link_idx: 0,
                gen_id: 0,
                scope_id,
            };

            let new_scope = Scope {
                sender: self.sender.clone(),
                parent_scope,
                our_arena_idx: scope_id,
                height,
                subtree: Cell::new(subtree),
                is_subtree_root: Cell::new(false),
                frames: [BumpFrame::new(), BumpFrame::new()],

                hooks: Default::default(),
                shared_contexts: Default::default(),
                caller,
                generation: 0.into(),

                old_root: RefCell::new(Some(old_root)),
                new_root: RefCell::new(Some(new_root)),
                // old_root: RefCell::new(Some(old_root)),
                // new_root: RefCell::new(None),
                items: RefCell::new(SelfReferentialItems {
                    listeners: Default::default(),
                    borrowed_props: Default::default(),
                    suspended_nodes: Default::default(),
                    tasks: Default::default(),
                    pending_effects: Default::default(),
                }),
            };

            let stable = self.bump.alloc(new_scope);
            self.scopes.push(stable);
            scope_id
        }
    }

    pub fn try_remove(&self, id: &ScopeId) -> Option<Scope> {
        todo!()
    }

    pub fn reserve_node(&self, node: &VNode) -> ElementId {
        let mut els = self.nodes.borrow_mut();
        let entry = els.vacant_entry();
        let key = entry.key();
        let id = ElementId(key);
        let node = node as *const _;
        let node = unsafe { std::mem::transmute(node) };
        entry.insert(node);
        id

        // let nodes = self.nodes.borrow_mut();
        // let id = nodes.insert(());
        // let node_id = ElementId(id);
        // node = Some(node_id);
        // node_id
    }

    pub fn collect_garbage(&self, id: ElementId) {
        todo!()
    }

    // These methods would normally exist on `scope` but they need access to *all* of the scopes

    /// This method cleans up any references to data held within our hook list. This prevents mutable aliasing from
    /// causing UB in our tree.
    ///
    /// This works by cleaning up our references from the bottom of the tree to the top. The directed graph of components
    /// essentially forms a dependency tree that we can traverse from the bottom to the top. As we traverse, we remove
    /// any possible references to the data in the hook list.
    ///
    /// References to hook data can only be stored in listeners and component props. During diffing, we make sure to log
    /// all listeners and borrowed props so we can clear them here.
    ///
    /// This also makes sure that drop order is consistent and predictable. All resources that rely on being dropped will
    /// be dropped.
    pub(crate) fn ensure_drop_safety(&self, scope_id: &ScopeId) {
        let scope = self.get_scope(scope_id).unwrap();

        // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
        // run the hooks (which hold an &mut Reference)
        // right now, we don't drop
        scope
            .items
            .borrow_mut()
            .borrowed_props
            .drain(..)
            .for_each(|comp| {
                // First drop the component's undropped references
                let scope_id = comp
                    .associated_scope
                    .get()
                    .expect("VComponents should be associated with a valid Scope");

                todo!("move this onto virtualdom");
                // let scope = unsafe { &mut *scope_id };

                // scope.ensure_drop_safety();

                todo!("drop the component's props");
                // let mut drop_props = comp.drop_props.borrow_mut().take().unwrap();
                // drop_props();
            });

        // Now that all the references are gone, we can safely drop our own references in our listeners.
        scope
            .items
            .borrow_mut()
            .listeners
            .drain(..)
            .map(|li| unsafe { &*li })
            .for_each(|listener| drop(listener.callback.borrow_mut().take()));
    }

    pub(crate) fn run_scope(&self, id: &ScopeId) -> bool {
        let scope = self
            .get_scope(id)
            .expect("The base scope should never be moved");

        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(id);

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        unsafe { scope.hooks.reset() };

        // Safety:
        // - We've dropped all references to the wip bump frame with "ensure_drop_safety"
        unsafe { scope.reset_wip_frame() };

        {
            let mut items = scope.items.borrow_mut();

            // just forget about our suspended nodes while we're at it
            items.suspended_nodes.clear();

            // guarantee that we haven't screwed up - there should be no latent references anywhere
            debug_assert!(items.listeners.is_empty());
            debug_assert!(items.suspended_nodes.is_empty());
            debug_assert!(items.borrowed_props.is_empty());

            log::debug!("Borrowed stuff is successfully cleared");

            // temporarily cast the vcomponent to the right lifetime
            // let vcomp = scope.load_vcomp();
        }

        let render: &dyn Fn(&Scope) -> Element = unsafe { &*scope.caller };

        // Todo: see if we can add stronger guarantees around internal bookkeeping and failed component renders.
        scope.wip_frame().nodes.borrow_mut().clear();
        if let Some(key) = render(scope) {
            dbg!(key);

            dbg!(&scope.wip_frame().nodes.borrow_mut());
            // let mut old = scope.old_root.borrow_mut();
            // let mut new = scope.new_root.borrow_mut();

            // let new_old = new.clone();
            // *old = new_old;
            // *new = Some(key);

            // dbg!(&old);
            // dbg!(&new);

            // the user's component succeeded. We can safely cycle to the next frame
            // scope.frames.wip_frame_mut().head_node = unsafe { std::mem::transmute(new_head) };
            scope.cycle_frame();

            true
        } else {
            false
        }
    }

    pub fn wip_head(&self, id: &ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let wip_frame = scope.wip_frame();
        let nodes = wip_frame.nodes.borrow();
        let node = nodes.get(0).unwrap();
        unsafe { std::mem::transmute(node) }
        // let root = scope.old_root.borrow();
        // let link = root.as_ref().unwrap();
        // dbg!(link);

        // // for now, components cannot pass portals through each other
        // assert_eq!(link.scope_id, *id);
        // // assert_eq!(link.gen_id, scope.generation.get() - 1);

        // // let items = scope.items.borrow();
        // let nodes = scope.wip_frame().nodes.borrow();
        // let node = nodes.get(link.link_idx).unwrap();
        // unsafe { std::mem::transmute(node) }
    }

    pub fn fin_head(&self, id: &ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let wip_frame = scope.fin_frame();
        let nodes = wip_frame.nodes.borrow();
        let node = nodes.get(0).unwrap();
        unsafe { std::mem::transmute(node) }
        // let scope = self.get_scope(id).unwrap();
        // let root = scope.new_root.borrow();
        // let link = root.as_ref().unwrap();

        // // for now, components cannot pass portals through each other
        // assert_eq!(link.scope_id, *id);
        // // assert_eq!(link.gen_id, scope.generation.get());

        // let nodes = scope.fin_frame().nodes.borrow();
        // let node = nodes.get(link.link_idx).unwrap();
        // unsafe { std::mem::transmute(node) }
    }
    pub fn root_node(&self, id: &ScopeId) -> &VNode {
        self.wip_head(id)
    }
}
