use std::{
    cell::{Ref, RefMut},
    fmt::Display,
    marker::PhantomData,
    ops::{Add, Div, Mul, Sub},
};

mod rt;

use dioxus_core::ScopeState;
pub use rt::*;

pub fn use_init_signal_rt(cx: &ScopeState) {
    cx.use_hook(|| {
        let rt = crate::rt::claim_rt(cx.schedule_update_any());
        cx.provide_context(rt);
    });
}

pub fn use_signal<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> Signal<T> {
    cx.use_hook(|| {
        let rt: &'static SignalRt = cx.consume_context().unwrap();
        let id = rt.init(f());
        rt.subscribe(id, cx.scope_id());

        struct SignalHook<T> {
            signal: Signal<T>,
        }

        impl<T> Drop for SignalHook<T> {
            fn drop(&mut self) {
                self.signal.rt.remove(self.signal.id);
            }
        }

        SignalHook {
            signal: Signal {
                id,
                rt,
                t: PhantomData,
            },
        }
    })
    .signal
}

pub struct Signal<T> {
    id: usize,
    rt: &'static SignalRt,
    t: PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    pub fn read(&self) -> Ref<T> {
        self.rt.read(self.id)
    }

    pub fn write(&self) -> RefMut<T> {
        self.rt.write(self.id)
    }

    pub fn set(&mut self, value: T) {
        self.rt.set(self.id, value);
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    pub fn update<O>(&self, _f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        _f(&mut *write)
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.rt.get(self.id)
    }
}

impl<T: Clone + 'static> std::ops::Deref for Signal<T> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        self.rt.getter(self.id)
    }
}

impl<T> std::clone::Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            t: PhantomData,
            id: self.id,
            rt: self.rt,
        }
    }
}

impl<T> Copy for Signal<T> {}

impl<T: Display + 'static> Display for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.rt.with::<T, _>(self.id, |v| T::fmt(v, f))
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
