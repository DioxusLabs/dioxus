use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use dioxus_core::ScopeState;

pub fn use_ref<'a, T: 'static>(cx: &'a ScopeState, f: impl FnOnce() -> T) -> &'a UseRef<T> {
    cx.use_hook(|_| UseRef {
        update_callback: cx.schedule_update(),
        value: Rc::new(RefCell::new(f())),
    })
}

pub struct UseRef<T> {
    update_callback: Rc<dyn Fn()>,
    value: Rc<RefCell<T>>,
}

impl<T> UseRef<T> {
    pub fn read(&self) -> Ref<'_, T> {
        self.value.borrow()
    }

    pub fn set(&self, new: T) {
        *self.value.borrow_mut() = new;
        self.needs_update();
    }

    pub fn read_write(&self) -> (Ref<'_, T>, &Self) {
        (self.read(), self)
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&self) -> RefMut<'_, T> {
        self.needs_update();
        self.value.borrow_mut()
    }

    /// Allows the ability to write the value without forcing a re-render
    pub fn write_silent(&self) -> RefMut<'_, T> {
        self.value.borrow_mut()
    }

    pub fn needs_update(&self) {
        (self.update_callback)();
    }
}

impl<T> Clone for UseRef<T> {
    fn clone(&self) -> Self {
        Self {
            update_callback: self.update_callback.clone(),
            value: self.value.clone(),
        }
    }
}
