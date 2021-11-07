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
}

impl ScopeArena {
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
            scopes: Vec::new(),
            free_scopes: Vec::new(),
        }
    }

    pub fn get(&self, id: &ScopeId) -> Option<&ScopeState> {
        unsafe { Some(&*self.scopes[id.0]) }
    }

    pub fn new_with_key(
        &mut self,
        fc_ptr: *const (),
        vcomp: &VComponent,
        parent_scope: Option<*mut ScopeState>,
        height: u32,
        subtree: u32,
        sender: UnboundedSender<SchedulerMsg>,
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
                sender,
                parent_scope,
                our_arena_idx: id,
                height,
                subtree: Cell::new(subtree),
                is_subtree_root: Cell::new(false),
                frames: ActiveFrame::new(),
                vcomp,

                hooks: Default::default(),
                shared_contexts: Default::default(),

                items: RefCell::new(SelfReferentialItems {
                    listeners: Default::default(),
                    borrowed_props: Default::default(),
                    suspended_nodes: Default::default(),
                    tasks: Default::default(),
                    pending_effects: Default::default(),
                    cached_nodes: Default::default(),
                    generation: Default::default(),
                }),
            };

            let stable = self.bump.alloc(new_scope);
            self.scopes.push(stable);
            id
        }
    }

    // scopes never get dropepd
}
