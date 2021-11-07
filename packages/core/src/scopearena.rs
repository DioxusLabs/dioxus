use std::cell::{Cell, RefCell};

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
    scopes: Vec<*mut ScopeState>,
    free_scopes: Vec<ScopeId>,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
}

impl ScopeArena {
    pub fn new(sender: UnboundedSender<SchedulerMsg>) -> Self {
        Self {
            bump: Bump::new(),
            scopes: Vec::new(),
            free_scopes: Vec::new(),
            sender,
        }
    }

    pub fn get_scope(&self, id: &ScopeId) -> Option<&ScopeState> {
        unsafe { Some(&*self.scopes[id.0]) }
    }

    pub fn new_with_key(
        &mut self,
        fc_ptr: *const (),
        vcomp: &VComponent,
        parent_scope: Option<*mut ScopeState>,
        height: u32,
        subtree: u32,
    ) -> ScopeId {
        if let Some(id) = self.free_scopes.pop() {
            // have already called drop on it - the slot is still chillin tho
            let scope = unsafe { &mut *self.scopes[id.0 as usize] };

            todo!("override the scope contents");
            id
        } else {
            let id = ScopeId(self.scopes.len());

            let vcomp = unsafe { std::mem::transmute(vcomp as *const VComponent) };

            let new_scope = ScopeState {
                sender: self.sender.clone(),
                parent_scope,
                our_arena_idx: id,
                height,
                subtree: Cell::new(subtree),
                is_subtree_root: Cell::new(false),
                frames: [Bump::default(), Bump::default()],
                vcomp,

                hooks: Default::default(),
                shared_contexts: Default::default(),

                items: RefCell::new(SelfReferentialItems {
                    listeners: Default::default(),
                    borrowed_props: Default::default(),
                    suspended_nodes: Default::default(),
                    tasks: Default::default(),
                    pending_effects: Default::default(),
                    cached_nodes_old: Default::default(),
                    generation: Default::default(),
                    cached_nodes_new: todo!(),
                }),
                old_root: todo!(),
                new_root: todo!(),
            };

            let stable = self.bump.alloc(new_scope);
            self.scopes.push(stable);
            id
        }
    }

    pub fn try_remove(&self, id: &ScopeId) -> Option<ScopeState> {
        todo!()
    }

    pub fn reserve_node(&self, node: &VNode) -> ElementId {
        todo!()
        // self.node_reservations.insert(id);
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
}
