use std::{
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
    *cx.use_hook(|| {
        let rt: &'static SignalRt = cx.consume_context().unwrap();
        let id = rt.init(f());
        rt.subscribe(id, cx.scope_id());

        Signal {
            rt,
            id,
            t: PhantomData,
        }
    })
}

pub struct Signal<T> {
    id: usize,
    rt: &'static SignalRt,
    t: PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    pub fn set(&mut self, value: T) {
        self.rt.set(self.id, value);
    }

    pub fn map<U>(&self, _f: impl FnOnce(T) -> U) -> Signal<U> {
        todo!()
    }

    pub fn update<O>(&self, _f: impl FnOnce(&mut T) -> O) {
        todo!()
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.rt.get(self.id)
    }
}

// impl<T> std::ops::Deref for Signal<T> {
//     type Target = dyn Fn() -> T;

//     fn deref(&self) -> &Self::Target {
//         todo!()
//     }
// }

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

// impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for Signal<T> {
//     fn add_assign(&mut self, rhs: T) {
//         self.set((*self.current()) + rhs);
//     }
// }

// impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for Signal<T> {
//     fn sub_assign(&mut self, rhs: T) {
//         self.set((*self.current()) - rhs);
//     }
// }

// impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for Signal<T> {
//     fn mul_assign(&mut self, rhs: T) {
//         self.set((*self.current()) * rhs);
//     }
// }

// impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for Signal<T> {
//     fn div_assign(&mut self, rhs: T) {
//         self.set((*self.current()) / rhs);
//     }
// }
