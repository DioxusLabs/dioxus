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

    pub fn try_get(&self, idx: ScopeIdx) -> Result<&Scope> {
        todo!()
    }

    pub fn try_get_mut(&self, idx: ScopeIdx) -> Result<&mut Scope> {
        todo!()
    }

    fn inner(&self) -> &Arena<Scope> {
        todo!()
    }

    fn inner_mut(&mut self) -> &mut Arena<Scope> {
        todo!()
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut Arena<Scope>) -> T) -> Result<T> {
        todo!()
    }

    unsafe fn inner_unchecked<'s>() -> &'s mut Arena<Scope> {
        todo!()
    }
}
