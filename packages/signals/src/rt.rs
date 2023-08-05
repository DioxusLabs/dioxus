use std::cell::{Ref, RefMut};

use std::rc::Rc;

use dioxus_core::prelude::{consume_context, provide_root_context};

use dioxus_copy::{CopyHandle, Owner, Store};

fn current_store() -> Store {
    match consume_context() {
        Some(rt) => rt,
        None => {
            let store = Store::default();
            provide_root_context(store).expect("in a virtual dom")
        }
    }
}

fn current_owner() -> Rc<Owner> {
    match consume_context() {
        Some(rt) => rt,
        None => {
            let owner = Rc::new(current_store().owner());
            provide_root_context(owner).expect("in a virtual dom")
        }
    }
}

pub struct CopyValue<T: 'static> {
    pub value: CopyHandle<T>,
}

impl<T: 'static> CopyValue<T> {
    pub fn new(value: T) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert(value),
        }
    }

    pub fn try_read(&self) -> Option<Ref<'_, T>> {
        self.value.try_read()
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.value.read()
    }

    pub fn try_write(&self) -> Option<RefMut<'_, T>> {
        self.value.try_write()
    }

    pub fn write(&self) -> RefMut<'_, T> {
        self.value.write()
    }

    pub fn set(&mut self, value: T) {
        *self.write() = value;
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }
}

impl<T: Clone + 'static> CopyValue<T> {
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl<T: 'static> PartialEq for CopyValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.ptr_eq(&other.value)
    }
}
