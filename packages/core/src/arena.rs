use std::{cell::UnsafeCell, rc::Rc};

use crate::innerlude::*;
use slotmap::SlotMap;

#[derive(Clone)]
pub struct SharedArena {
    pub components: Rc<UnsafeCell<ScopeMap>>,
}
pub type ScopeMap = SlotMap<ScopeId, Scope>;

enum MutStatus {
    Immut,
    Mut,
}

impl SharedArena {
    pub fn new(arena: ScopeMap) -> Self {
        let components = Rc::new(UnsafeCell::new(arena));
        SharedArena { components }
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn get(&self, idx: ScopeId) -> Option<&Scope> {
        let inner = unsafe { &*self.components.get() };
        inner.get(idx)
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn get_mut(&self, idx: ScopeId) -> Option<&mut Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner.get_mut(idx)
    }

    fn inner(&self) -> &ScopeMap {
        todo!()
    }

    fn inner_mut(&mut self) -> &mut ScopeMap {
        todo!()
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn with<T>(&self, f: impl FnOnce(&mut ScopeMap) -> T) -> Result<T> {
        let inner = unsafe { &mut *self.components.get() };
        Ok(f(inner))
        // todo!()
    }

    pub fn with_scope<'b, O: 'static>(
        &'b self,
        _id: ScopeId,
        _f: impl FnOnce(&'b mut Scope) -> O,
    ) -> Result<O> {
        todo!()
    }

    // return a bumpframe with a lifetime attached to the arena borrow
    // this is useful for merging lifetimes
    pub fn with_scope_vnode<'b>(
        &self,
        _id: ScopeId,
        _f: impl FnOnce(&mut Scope) -> &VNode<'b>,
    ) -> Result<&VNode<'b>> {
        todo!()
    }

    pub fn try_remove(&self, id: ScopeId) -> Result<Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner
            .remove(id)
            .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    unsafe fn inner_unchecked<'s>() -> &'s mut ScopeMap {
        todo!()
    }
}
