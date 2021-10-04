use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    rc::Rc,
};

use dioxus_core::Context;

pub fn use_ref<T: 'static>(cx: Context, f: impl FnOnce() -> T) -> UseRef<T> {
    cx.use_hook(
        |_| UseRefInner {
            update_scheuled: Cell::new(false),
            update_callback: cx.schedule_update(),
            value: RefCell::new(f()),
        },
        |inner| {
            inner.update_scheuled.set(false);
            UseRef { inner }
        },
        |_| {},
    )
}

pub struct UseRef<'a, T> {
    inner: &'a UseRefInner<T>,
}
struct UseRefInner<T> {
    update_scheuled: Cell<bool>,
    update_callback: Rc<dyn Fn()>,
    value: RefCell<T>,
}

impl<'a, T> UseRef<'a, T> {
    pub fn read(&self) -> Ref<'_, T> {
        self.inner.value.borrow()
    }

    pub fn read_write(&self) -> (Ref<'_, T>, &Self) {
        (self.read(), self)
    }

    /// Calling "write" will force the component to re-render
    pub fn write(&self) -> RefMut<'_, T> {
        self.needs_update();
        self.inner.value.borrow_mut()
    }

    /// Allows the ability to write the value without forcing a re-render
    pub fn write_silent(&self) -> RefMut<'_, T> {
        self.inner.value.borrow_mut()
    }

    pub fn needs_update(&self) {
        if !self.inner.update_scheuled.get() {
            self.inner.update_scheuled.set(true);
            (self.inner.update_callback)();
        }
    }
}

impl<T> Clone for UseRef<'_, T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}
impl<T> Copy for UseRef<'_, T> {}
