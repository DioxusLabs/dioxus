use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

use crate::innerlude::*;
use slotmap::{DefaultKey, SlotMap};

#[derive(Clone)]
pub struct SharedArena {
    pub components: Arc<UnsafeCell<ScopeMap>>,
}
pub type ScopeMap = SlotMap<DefaultKey, Scope>;

enum MutStatus {
    Immut,
    Mut,
}

impl SharedArena {
    pub fn new(arena: ScopeMap) -> Self {
        let components = Arc::new(UnsafeCell::new(arena));
        SharedArena { components }
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn try_get(&self, idx: ScopeId) -> Result<&Scope> {
        let inner = unsafe { &*self.components.get() };
        let scope = inner.get(idx);
        scope.ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn try_get_mut(&self, idx: ScopeId) -> Result<&mut Scope> {
        let inner = unsafe { &mut *self.components.get() };
        let scope = inner.get_mut(idx);
        scope.ok_or_else(|| Error::FatalInternal("Scope not found"))
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
