use crate::innerlude::*;
use slab::Slab;

use std::{cell::UnsafeCell, rc::Rc};
#[derive(Clone)]
pub(crate) struct ResourcePool {
    /*
    This *has* to be an UnsafeCell.

    Each BumpFrame and Scope is located in this Slab - and we'll need mutable access to a scope while holding on to
    its bumpframe contents immutably.

    However, all of the interaction with this Slab is done in this module and the Diff module, so it should be fairly
    simple to audit.

    Wrapped in Rc so the "get_shared_context" closure can walk the tree (immutably!)
    */
    pub components: Rc<UnsafeCell<Slab<Scope>>>,

    /*
    Yes, a slab of "nil". We use this for properly ordering ElementIDs - all we care about is the allocation strategy
    that slab uses. The slab essentially just provides keys for ElementIDs that we can re-use in a Vec on the client.

    This just happened to be the simplest and most efficient way to implement a deterministic keyed map with slot reuse.

    In the future, we could actually store a pointer to the VNode instead of nil to provide O(1) lookup for VNodes...
    */
    pub raw_elements: Rc<UnsafeCell<Slab<*const VNode<'static>>>>,

    pub channel: EventChannel,
}

impl ResourcePool {
    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope(&self, idx: ScopeId) -> Option<&Scope> {
        let inner = unsafe { &*self.components.get() };
        inner.get(idx.0)
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope_mut(&self, idx: ScopeId) -> Option<&mut Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner.get_mut(idx.0)
    }

    pub fn try_remove(&self, id: ScopeId) -> Option<Scope> {
        let inner = unsafe { &mut *self.components.get() };
        Some(inner.remove(id.0))
        // .try_remove(id.0)
        // .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    pub fn reserve_node<'a>(&self, node: &'a VNode<'a>) -> ElementId {
        let els = unsafe { &mut *self.raw_elements.get() };
        let entry = els.vacant_entry();
        let key = entry.key();
        let id = ElementId(key);
        let node = node as *const _;
        let node = unsafe { std::mem::transmute(node) };
        entry.insert(node);
        id
    }

    /// return the id, freeing the space of the original node
    pub fn collect_garbage(&self, id: ElementId) {
        let els = unsafe { &mut *self.raw_elements.get() };
        els.remove(id.0);
    }

    pub fn insert_scope_with_key(&self, f: impl FnOnce(ScopeId) -> Scope) -> ScopeId {
        let g = unsafe { &mut *self.components.get() };
        let entry = g.vacant_entry();
        let id = ScopeId(entry.key());
        entry.insert(f(id));
        id
    }
}
