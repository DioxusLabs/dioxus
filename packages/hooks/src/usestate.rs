use dioxus_core::prelude::Context;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Store state between component renders!
///
/// ## The "King" of state hooks
///
/// The Dioxus version of `useState` is the "king daddy" of state management. It allows you to ergonomically store and
/// modify state between component renders. When the state is updated, the component will re-render.
///
/// Dioxus' use_state basically wraps a RefCell with helper methods and integrates it with the VirtualDOM update system.
///
/// [`use_state`] exposes a few helper methods to modify the underlying state:
/// - `.set(new)` allows you to override the "work in progress" value with a new value
/// - `.get_mut()` allows you to modify the WIP value
/// - `.get_wip()` allows you to access the WIP value
/// - `.deref()` provides the previous value (often done implicitly, though a manual dereference with `*` might be required)
///
/// Additionally, a ton of std::ops traits are implemented for the `UseState` wrapper, meaning any mutative type operations
/// will automatically be called on the WIP value.
///
///
/// Usage:
/// ```ignore
/// const Example: FC<()> = |cx| {
///     let counter = use_state(cx, || 0);
///     let increment = |_| counter += 1;
///     let decrement = |_| counter += 1;
///
///     html! {
///         <div>
///             <h1>"Counter: {counter}" </h1>
///             <button onclick={increment}> "Increment" </button>
///             <button onclick={decrement}> "Decrement" </button>
///         </div>  
///     }
/// }
/// ```
pub fn use_state<'a, 'c, T: 'static, F: FnOnce() -> T, P>(
    cx: Context<'a, P>,
    initial_state_fn: F,
) -> UseState<T> {
    cx.use_hook(
        move || UseStateInner {
            current_val: initial_state_fn(),
            callback: cx.schedule_update(),
            wip: RefCell::new(None),
            update_scheuled: Cell::new(false),
        },
        move |hook| {
            hook.update_scheuled.set(false);
            let mut new_val = hook.wip.borrow_mut();
            if new_val.is_some() {
                hook.current_val = new_val.take().unwrap();
            }

            UseState { inner: &*hook }
        },
        |_| {},
    )
}
struct UseStateInner<T: 'static> {
    current_val: T,
    update_scheuled: Cell<bool>,
    callback: Rc<dyn Fn()>,
    wip: RefCell<Option<T>>,
    updater: 
}

pub struct UseState<'a, T: 'static> {
    inner: &'a UseStateInner<T>,
}
impl<T> Copy for UseState<'_, T> {}
impl<'a, T> Clone for UseState<'a, T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        UseState { inner: self.inner }
    }
}

impl<'a, T: 'static> UseState<'a, T> {
    /// Tell the Dioxus Scheduler that we need to be processed
    pub fn needs_update(&self) {
        if !self.inner.update_scheuled.get() {
            self.inner.update_scheuled.set(true);
            (self.inner.callback)();
        }
    }

    pub fn set(&self, new_val: T) {
        self.needs_update();
        *self.inner.wip.borrow_mut() = Some(new_val);
    }

    pub fn get(&self) -> &T {
        &self.inner.current_val
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip(&self) -> Ref<Option<T>> {
        self.inner.wip.borrow()
    }

    pub fn classic(self) -> (&'a T, &'a Rc<dyn Fn(T)>) {
        (&self.inner.current_val)
    }
}
impl<'a, T: 'static + ToOwned<Owned = T>> UseState<'a, T> {
    pub fn get_mut(self) -> RefMut<'a, T> {
        // make sure we get processed
        self.needs_update();

        // Bring out the new value, cloning if it we need to
        // "get_mut" is locked behind "ToOwned" to make it explicit that cloning occurs to use this
        RefMut::map(self.inner.wip.borrow_mut(), |slot| {
            if slot.is_none() {
                *slot = Some(self.inner.current_val.to_owned());
            }
            slot.as_mut().unwrap()
        })
    }
}

impl<'a, T: 'static> std::ops::Deref for UseState<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.current_val
    }
}

use std::ops::{Add, AddAssign, Sub, SubAssign};
impl<'a, T: Copy + Add<T, Output = T>> Add<T> for UseState<'a, T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.inner.current_val.add(rhs)
    }
}
impl<'a, T: Copy + Add<T, Output = T>> AddAssign<T> for UseState<'a, T> {
    fn add_assign(&mut self, rhs: T) {
        self.set(self.inner.current_val.add(rhs));
    }
}
impl<'a, T: Copy + Sub<T, Output = T>> Sub<T> for UseState<'a, T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.inner.current_val.sub(rhs)
    }
}
impl<'a, T: Copy + Sub<T, Output = T>> SubAssign<T> for UseState<'a, T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set(self.inner.current_val.sub(rhs));
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.current_val)
    }
}
