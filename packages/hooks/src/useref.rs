use std::cell::{Ref, RefCell, RefMut};

use dioxus_core::Context;

pub struct UseRef<'a, T> {
    inner: &'a RefCell<T>,
}

impl<'a, T> UseRef<'a, T> {
    pub fn read(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    pub fn read_write(&self) -> (Ref<'_, T>, &Self) {
        (self.read(), self)
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    /// Allows the ability to write the value without forcing a re-render
    pub fn write_silent(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

impl<T> Clone for UseRef<'_, T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
impl<T> Copy for UseRef<'_, T> {}

pub fn use_ref<P, T: 'static>(cx: Context<P>, f: impl FnOnce() -> T) -> UseRef<T> {
    cx.use_hook(|_| RefCell::new(f()), |f| UseRef { inner: f }, |_| {})
}
