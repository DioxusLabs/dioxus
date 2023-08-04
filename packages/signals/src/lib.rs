use std::{
    cell::{Ref, RefMut},
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
    sync::Arc,
};

mod copy;
mod rt;
pub use rt::*;

use dioxus_core::{
    prelude::{current_scope_id, schedule_update_any},
    ScopeId, ScopeState,
};

pub fn use_signal<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> Signal<T> {
    *cx.use_hook(|| Signal::new(f()))
}

struct SignalData<T> {
    subscribers: Vec<ScopeId>,
    update_any: Arc<dyn Fn(ScopeId)>,
    value: T,
}

pub struct Signal<T: 'static> {
    inner: CopyValue<SignalData<T>>,
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: CopyValue::new(SignalData {
                subscribers: Vec::new(),
                update_any: schedule_update_any().expect("in a virtual dom"),
                value,
            }),
        }
    }

    pub fn read(&self) -> Ref<T> {
        if let Some(current_scope_id) = current_scope_id() {
            let mut inner = self.inner.write();
            if !inner.subscribers.contains(&current_scope_id) {
                inner.subscribers.push(current_scope_id);
            }
        }
        Ref::map(self.inner.read(), |v| &v.value)
    }

    pub fn write(&self) -> RefMut<T> {
        let inner = self.inner.write();
        for &scope_id in &inner.subscribers {
            (inner.update_any)(scope_id);
        }

        RefMut::map(inner, |v| &mut v.value)
    }

    pub fn set(&mut self, value: T) {
        *self.write() = value;
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    pub fn update<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.read().clone()
    }
}

impl<T> std::clone::Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Signal<T> {}

impl<T: Display + 'static> Display for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| Display::fmt(v, f))
    }
}

impl<T: Add<Output = T> + Copy + 'static> std::ops::AddAssign<T> for Signal<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set(self.get() + rhs);
    }
}

impl<T: Sub<Output = T> + Copy + 'static> std::ops::SubAssign<T> for Signal<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set(self.get() - rhs);
    }
}

impl<T: Mul<Output = T> + Copy + 'static> std::ops::MulAssign<T> for Signal<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set(self.get() * rhs);
    }
}

impl<T: Div<Output = T> + Copy + 'static> std::ops::DivAssign<T> for Signal<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set(self.get() / rhs);
    }
}
