use std::{cell::UnsafeCell, collections::HashMap};

use generational_arena::Arena;

use crate::innerlude::*;

pub struct ScopeArena {
    pub(crate) arena: UnsafeCell<Arena<Scope>>,
    locks: HashMap<ScopeIdx, MutStatus>,
}

enum MutStatus {
    Immut,
    Mut,
}

impl ScopeArena {
    pub fn new(arena: Arena<Scope>) -> Self {
        Self {
            arena: UnsafeCell::new(arena),
            locks: Default::default(),
        }
    }

    pub fn get(&self, idx: ScopeIdx) -> Result<&Scope> {
        todo!()
    }

    pub fn get_mut(&self, idx: ScopeIdx) -> Result<&Scope> {
        todo!()
    }

    fn inner(&self) -> &Arena<Scope> {
        todo!()
    }

    fn inner_mut(&mut self) -> &mut Arena<Scope> {
        todo!()
    }

    pub fn with<T>(&mut self, f: impl FnOnce(&mut Arena<Scope>) -> T) -> T {
        todo!()
    }

    unsafe fn inner_unchecked<'s>() -> &'s mut Arena<Scope> {
        todo!()
    }
}
