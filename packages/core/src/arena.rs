use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
};

use crate::innerlude::*;
use slotmap::{DefaultKey, SlotMap};

#[derive(Clone)]
pub struct ScopeArena(pub Rc<RefCell<ScopeArenaInner>>);
pub type ScopeMap = SlotMap<DefaultKey, Scope>;

pub struct ScopeArenaInner {
    pub(crate) arena: UnsafeCell<ScopeMap>,
    locks: HashMap<ScopeIdx, MutStatus>,
}

enum MutStatus {
    Immut,
    Mut,
}

// impl ScopeArenaInner {
//     pub fn new(arena: Arena<Scope>) -> Self {
//         ScopeArenaInner {
//             arena: UnsafeCell::new(arena),
//             locks: Default::default(),
//         }
//     }

//     /// THIS METHOD IS CURRENTLY UNSAFE
//     /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
//     pub fn try_get(&self, idx: ScopeIdx) -> Result<&Scope> {
//         let inner = unsafe { &*self.arena.get() };
//         let scope = inner.get(idx);
//         scope.ok_or_else(|| Error::FatalInternal("Scope not found"))
//     }

//     /// THIS METHOD IS CURRENTLY UNSAFE
//     /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
//     pub fn try_get_mut(&self, idx: ScopeIdx) -> Result<&mut Scope> {
//         let inner = unsafe { &mut *self.arena.get() };
//         let scope = inner.get_mut(idx);
//         scope.ok_or_else(|| Error::FatalInternal("Scope not found"))
//     }

//     fn inner(&self) -> &Arena<Scope> {
//         todo!()
//     }

//     fn inner_mut(&mut self) -> &mut Arena<Scope> {
//         todo!()
//     }

//     /// THIS METHOD IS CURRENTLY UNSAFE
//     /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
//     pub fn with<T>(&self, f: impl FnOnce(&mut Arena<Scope>) -> T) -> Result<T> {
//         let inner = unsafe { &mut *self.arena.get() };
//         Ok(f(inner))
//         // todo!()
//     }

//     pub fn with_scope<'b, O: 'static>(
//         &'b self,
//         id: ScopeIdx,
//         f: impl FnOnce(&'b mut Scope) -> O,
//     ) -> Result<O> {
//         todo!()
//     }

//     // return a bumpframe with a lifetime attached to the arena borrow
//     // this is useful for merging lifetimes
//     pub fn with_scope_vnode<'b>(
//         &self,
//         id: ScopeIdx,
//         f: impl FnOnce(&mut Scope) -> &VNode<'b>,
//     ) -> Result<&VNode<'b>> {
//         todo!()
//     }

//     pub fn try_remove(&mut self, id: ScopeIdx) -> Result<Scope> {
//         let inner = unsafe { &mut *self.arena.get() };
//         inner
//             .remove(id)
//             .ok_or_else(|| Error::FatalInternal("Scope not found"))
//     }

//     unsafe fn inner_unchecked<'s>() -> &'s mut Arena<Scope> {
//         todo!()
//     }
// }
impl ScopeArena {
    pub fn new(arena: ScopeMap) -> Self {
        ScopeArena(Rc::new(RefCell::new(ScopeArenaInner {
            arena: UnsafeCell::new(arena),
            locks: Default::default(),
        })))
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn try_get(&self, idx: ScopeIdx) -> Result<&Scope> {
        let inner = unsafe { &*self.0.borrow().arena.get() };
        let scope = inner.get(idx);
        scope.ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn try_get_mut(&self, idx: ScopeIdx) -> Result<&mut Scope> {
        let inner = unsafe { &mut *self.0.borrow().arena.get() };
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
        let inner = unsafe { &mut *self.0.borrow().arena.get() };
        Ok(f(inner))
        // todo!()
    }

    pub fn with_scope<'b, O: 'static>(
        &'b self,
        id: ScopeIdx,
        f: impl FnOnce(&'b mut Scope) -> O,
    ) -> Result<O> {
        todo!()
    }

    // return a bumpframe with a lifetime attached to the arena borrow
    // this is useful for merging lifetimes
    pub fn with_scope_vnode<'b>(
        &self,
        id: ScopeIdx,
        f: impl FnOnce(&mut Scope) -> &VNode<'b>,
    ) -> Result<&VNode<'b>> {
        todo!()
    }

    pub fn try_remove(&self, id: ScopeIdx) -> Result<Scope> {
        let inner = unsafe { &mut *self.0.borrow().arena.get() };
        inner
            .remove(id)
            .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    unsafe fn inner_unchecked<'s>() -> &'s mut ScopeMap {
        todo!()
    }
}
