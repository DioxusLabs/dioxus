use core::hash::Hash;
use dioxus_core::ScopeState;
use generational_arena::Index;
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    rc::Rc,
};

mod rt;

pub use rt::*;

pub fn use_init_signal_rt(cx: &ScopeState) {
    cx.use_hook(|| {
        let owner = RuntimeOwner::new(cx.schedule_update_any());
        cx.provide_context(*owner);
        owner
    });
}

pub fn use_signal<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> Signal<T> {
    cx.use_hook(|| {
        let rt_id: RunTimeId = cx.consume_context().unwrap();
        let id = with_rt(rt_id, move |rt| {
            let id = rt.init(f());
            rt.subscribe(id, cx.scope_id());
            id
        });

        struct SignalHook<T> {
            signal: Signal<T>,
        }

        impl<T> Drop for SignalHook<T> {
            fn drop(&mut self) {
                try_with_rt(self.signal.rt_id, |rt| {
                    rt.remove(self.signal.id);
                });
            }
        }

        SignalHook {
            signal: Signal {
                id,
                rt_id,
                t: PhantomData,
            },
        }
    })
    .signal
}

pub struct Signal<T> {
    id: Index,
    rt_id: RunTimeId,
    t: PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    /// Create a new signal in a specific runtime. The value will not be dropped until the runtime is dropped.
    ///
    /// This is useful largely for testing. For use in Dioxus, use `use_signal` which will drop the signal when the scope is dropped.
    pub fn new_in(rt_id: RunTimeId, value: T) -> Self {
        let id = with_rt(rt_id, |rt| rt.init(value));
        Self {
            id,
            rt_id,
            t: PhantomData,
        }
    }

    #[inline(always)]
    fn with_rt<R>(&self, f: impl FnOnce(&SignalRt) -> R) -> R {
        with_rt(self.rt_id, f)
    }

    pub fn set(&mut self, value: T) {
        self.with_rt(|rt| rt.set(self.id, value))
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.with_rt(|rt| rt.with(self.id, f))
    }

    pub fn update<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        self.with_rt(|rt| rt.update(self.id, f))
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.with_rt(|rt| rt.get(self.id))
    }

    pub fn getter(&self) -> Rc<dyn Fn() -> T> {
        self.with_rt(|rt| rt.getter(self.id))
    }
}

impl<T> std::clone::Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            t: PhantomData,
            id: self.id,
            rt_id: self.rt_id,
        }
    }
}

impl<T> Copy for Signal<T> {}

impl<T: Display + 'static> Display for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| T::fmt(v, f))
    }
}

impl<T: Debug + 'static> Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| T::fmt(v, f))
    }
}

impl<T: AddAssign<T> + 'static> std::ops::AddAssign<T> for Signal<T> {
    fn add_assign(&mut self, rhs: T) {
        self.update(|v| *v += rhs);
    }
}

impl<T: Add<Output = T> + Clone + 'static> std::ops::Add<T> for Signal<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: SubAssign<T> + 'static> std::ops::SubAssign<T> for Signal<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.update(|v| *v -= rhs);
    }
}

impl<T: Sub<Output = T> + Clone + 'static> std::ops::Sub<T> for Signal<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: MulAssign<T> + 'static> std::ops::MulAssign<T> for Signal<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.update(|v| *v *= rhs);
    }
}

impl<T: Mul<Output = T> + Clone + 'static> std::ops::Mul<T> for Signal<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: DivAssign<T> + 'static> std::ops::DivAssign<T> for Signal<T> {
    fn div_assign(&mut self, rhs: T) {
        self.update(|v| *v /= rhs);
    }
}

impl<T: Div<Output = T> + Clone + 'static> std::ops::Div<T> for Signal<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: PartialEq + 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.with(|v| other.with(|v2| v == v2))
    }
}

impl<T: Eq + 'static> Eq for Signal<T> {}

impl<T: PartialEq + 'static> PartialEq<T> for Signal<T> {
    fn eq(&self, other: &T) -> bool {
        self.with(|v| v == other)
    }
}

impl<T: PartialOrd + 'static> PartialOrd for Signal<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.with(|v| other.with(|v2| v.partial_cmp(v2)))
    }
}

impl<T: Ord + 'static> Ord for Signal<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.with(|v| other.with(|v2| v.cmp(v2)))
    }
}

impl<T: PartialOrd + 'static> PartialOrd<T> for Signal<T> {
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.with(|v| v.partial_cmp(other))
    }
}

impl<T: Hash + 'static> Hash for Signal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.with(|v| v.hash(state));
    }
}
