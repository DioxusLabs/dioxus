#![doc = include_str!("../README.md")]

// pub mod prelude {
//     pub use crate::*;
// }

// mod root;

// pub use atoms::*;
// pub use hooks::*;
// pub use root::*;

// mod atoms {
//     mod atom;
//     mod atomfamily;
//     mod atomref;
//     mod selector;
//     mod selectorfamily;

//     pub use atom::*;
//     pub use atomfamily::*;
//     pub use atomref::*;
//     pub use selector::*;
//     pub use selectorfamily::*;
// }

// pub mod hooks {
//     mod atom_ref;
//     mod atom_root;
//     mod init_atom_root;
//     mod read;
//     mod set;
//     mod state;
//     pub use atom_ref::*;
//     pub use atom_root::*;
//     pub use init_atom_root::*;
//     pub use read::*;
//     pub use set::*;
//     pub use state::*;
// }

use std::{any::Any, cell::Ref, collections::HashMap, rc::Rc};

use dioxus_signals::Signal;

pub struct AtomRoot {
    pub atoms: HashMap<*const (), Box<dyn Any>>,
}

pub fn consume_root_context() -> Rc<AtomRoot> {
    dioxus_core::prelude::consume_context().unwrap()
}

pub struct Atom<T>(pub fn(AtomBuilder) -> T);
impl<T> Clone for Atom<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<T> Copy for Atom<T> {}

impl<T: PartialEq + 'static> std::cmp::PartialEq<T> for Atom<T> {
    fn eq(&self, other: &T) -> bool {
        self.value().with(|f| f == other)
    }
}

impl<T: 'static> std::ops::Index<usize> for Atom<Vec<T>> {
    type Output = Signal<T>;

    fn index(&self, index: usize) -> &Self::Output {
        todo!()
    }
}

pub struct AtomBuilder;

impl<T: 'static> Atom<T> {
    pub fn value(&self) -> Signal<T> {
        let id = self as &Atom<T> as *const Atom<T> as *const _;
        let root = consume_root_context();
        let slot = root.atoms.get(&id).unwrap().as_ref();
        let atom: &Signal<T> = slot.downcast_ref().unwrap();
        atom.clone()
    }

    pub fn set(&self, value: T) {
        let sig = self.value();
        sig.set(value);
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let sig = self.value();
        sig.with(f)
    }

    pub fn select<V>(&self, f: impl FnMut(&T) -> V + 'static) -> V {
        todo!()
    }
}

impl<T: 'static + Clone> Atom<T> {
    pub fn get(&self) -> T {
        self.value().read().clone()
    }
}
