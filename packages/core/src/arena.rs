use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
};

use generational_arena::Arena;

use crate::innerlude::*;

#[derive(Clone)]
pub struct ScopeArena(Rc<RefCell<ScopeArenaInner>>);

struct ScopeArenaInner {
    pub(crate) arena: UnsafeCell<Arena<Scope>>,
    locks: HashMap<ScopeIdx, MutStatus>,
}

enum MutStatus {
    Immut,
    Mut,
}

impl ScopeArena {
    pub fn new(arena: Arena<Scope>) -> Self {
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

    fn inner(&self) -> &Arena<Scope> {
        todo!()
    }

    fn inner_mut(&mut self) -> &mut Arena<Scope> {
        todo!()
    }

    /// THIS METHOD IS CURRENTLY UNSAFE
    /// THERE ARE NO CHECKS TO VERIFY THAT WE ARE ALLOWED TO DO THIS
    pub fn with<T>(&self, f: impl FnOnce(&mut Arena<Scope>) -> T) -> Result<T> {
        let inner = unsafe { &mut *self.0.borrow().arena.get() };
        Ok(f(inner))
        // todo!()
    }

    unsafe fn inner_unchecked<'s>() -> &'s mut Arena<Scope> {
        todo!()
    }
}
