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
#[must_use]
pub fn use_atom_ref<'a, T: 'static>(
    cx: &'a ScopeState,
    atom: &'static AtomRef<T>,
) -> &'a UseAtomRef<T> {
    let root = use_atom_root(cx);

    &cx.use_hook(|| {
        root.initialize(atom);
        (
            UseAtomRef {
                ptr: atom.unique_id(),
                root: root.clone(),
                scope_id: cx.scope_id(),
                value: root.register(atom, cx.scope_id()),
            },
            AtomRefSubscription {
                ptr: atom.unique_id(),
                root: root.clone(),
                scope_id: cx.scope_id(),
            },
        )
    })
    .0
}

pub struct AtomRefSubscription {
    ptr: AtomId,
    root: Rc<AtomRoot>,
    scope_id: ScopeId,
}

impl Drop for AtomRefSubscription {
    fn drop(&mut self) {
        self.root.unsubscribe(self.ptr, self.scope_id)
    }
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

    /// This is silent operation
    /// call `.force_update()` manually if required
    pub fn with_mut_silent(&self, cb: impl FnOnce(&mut T)) {
        cb(&mut *self.write_silent())
    }

    pub fn write(&self) -> RefMut<T> {
        self.root.force_update(self.ptr);
        self.value.borrow_mut()
    }

    /// Silent write to AtomRef
    /// does not update Subscribed scopes
    pub fn write_silent(&self) -> RefMut<T> {
        self.value.borrow_mut()
    }

    /// Replace old value with new one
    pub fn set(&self, new: T) {
        self.root.force_update(self.ptr);
        self.root.set(self.ptr, new);
    }

    /// Do not update provided context on Write ops
    /// Example:
    /// ```ignore
    /// static ATOM_DATA: AtomRef<Collection> = |_| Default::default();
    /// fn App(cx: Scope) {
    ///     use_init_atom_root(cx);
    ///     let atom_data = use_atom_ref(cx, ATOM_DATA);
    ///     atom_data.unsubscribe(cx);
    ///     atom_data.write().update();
    /// }
    /// ```
    pub fn unsubscribe(&self, cx: &ScopeState) {
        self.root.unsubscribe(self.ptr, cx.scope_id());
    }

    /// Force update of subscribed Scopes
    pub fn force_update(&self) {
        self.root.force_update(self.ptr);
    }
}
