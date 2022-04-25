use crate::{use_atom_root, AtomId, AtomRef, AtomRoot, Readable};
use dioxus_core::{ScopeId, ScopeState};
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

///
///
///
///
///
///
///
///
pub fn use_atom_ref<T: 'static>(cx: &ScopeState, atom: AtomRef<T>) -> &UseAtomRef<T> {
    let root = use_atom_root(cx);

    cx.use_hook(|_| {
        root.initialize(atom);
        UseAtomRef {
            ptr: atom.unique_id(),
            root: root.clone(),
            scope_id: cx.scope_id(),
            value: root.register(atom, cx.scope_id()),
        }
    })
}

pub struct UseAtomRef<T> {
    ptr: AtomId,
    value: Rc<RefCell<T>>,
    root: Rc<AtomRoot>,
    scope_id: ScopeId,
}

impl<T> Clone for UseAtomRef<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            value: self.value.clone(),
            root: self.root.clone(),
            scope_id: self.scope_id,
        }
    }
}

impl<T: 'static> UseAtomRef<T> {
    pub fn read(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn write(&self) -> RefMut<T> {
        self.root.force_update(self.ptr);
        self.value.borrow_mut()
    }

    pub fn write_silent(&self) -> RefMut<T> {
        self.root.force_update(self.ptr);
        self.value.borrow_mut()
    }

    pub fn set(&self, new: T) {
        self.root.force_update(self.ptr);
        self.root.set(self.ptr, new);
    }
}

impl<T> Drop for UseAtomRef<T> {
    fn drop(&mut self) {
        self.root.unsubscribe(self.ptr, self.scope_id)
    }
}
