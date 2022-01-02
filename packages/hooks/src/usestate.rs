use dioxus_core::prelude::*;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::{Debug, Display},
    rc::Rc,
};

/// Store state between component renders!
///
/// ## Dioxus equivalent of useState, designed for Rust
///
/// The Dioxus version of `useState` for state management inside components. It allows you to ergonomically store and
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
/// ## Combinators
///
/// On top of the methods to set/get state, `use_state` also supports fancy combinators to extend its functionality:
/// - `.classic()` and `.split()`  convert the hook into the classic React-style hook
///     ```rust
///     let (state, set_state) = use_state(&cx, || 10).split()
///     ```
///
///
/// Usage:
///
/// ```ignore
/// const Example: Component = |cx| {
///     let counter = use_state(&cx, || 0);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Counter: {counter}" }
///             button { onclick: move |_| counter += 1, "Increment" }
///             button { onclick: move |_| counter -= 1, "Decrement" }
///         }
///     ))
/// }
/// ```
pub fn use_state<'a, T: 'static>(
    cx: &'a ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> UseState<'a, T> {
    let hook = cx.use_hook(move |_| {
        let first_val = initial_state_fn();
        UseStateInner {
            current_val: Rc::new(first_val),
            update_callback: cx.schedule_update(),
            wip: Rc::new(RefCell::new(None)),
            update_scheuled: Cell::new(false),
        }
    });

    hook.update_scheuled.set(false);
    let mut new_val = hook.wip.borrow_mut();
    if new_val.is_some() {
        // if there's only one reference (weak or otherwise), we can just swap the values
        if let Some(val) = Rc::get_mut(&mut hook.current_val) {
            *val = new_val.take().unwrap();
        } else {
            hook.current_val = Rc::new(new_val.take().unwrap());
        }
    }

    UseState { inner: &*hook }
}
struct UseStateInner<T: 'static> {
    current_val: Rc<T>,
    update_scheuled: Cell<bool>,
    update_callback: Rc<dyn Fn()>,
    wip: Rc<RefCell<Option<T>>>,
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

impl<T: Debug> Debug for UseState<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner.current_val)
    }
}
impl<'a, T: 'static> UseState<'a, T> {
    /// Tell the Dioxus Scheduler that we need to be processed
    pub fn needs_update(&self) {
        if !self.inner.update_scheuled.get() {
            self.inner.update_scheuled.set(true);
            (self.inner.update_callback)();
        }
    }

    pub fn set(&self, new_val: T) {
        *self.inner.wip.borrow_mut() = Some(new_val);
        self.needs_update();
    }

    pub fn get(&self) -> &'a T {
        &self.inner.current_val
    }

    pub fn get_rc(&self) -> &'a Rc<T> {
        &self.inner.current_val
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip(&self) -> Ref<Option<T>> {
        self.inner.wip.borrow()
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip_mut(&self) -> RefMut<Option<T>> {
        self.inner.wip.borrow_mut()
    }

    pub fn classic(self) -> (&'a T, Rc<dyn Fn(T)>) {
        (&self.inner.current_val, self.setter())
    }

    pub fn setter(&self) -> Rc<dyn Fn(T)> {
        let slot = self.inner.wip.clone();
        Rc::new(move |new| {
            *slot.borrow_mut() = Some(new);
        })
    }

    pub fn for_async(self) -> UseState<'static, T> {
        todo!()
    }

    pub fn wtih(self, f: impl FnOnce(&mut T)) {
        let mut val = self.inner.wip.borrow_mut();

        if let Some(inner) = val.as_mut() {
            f(inner);
        }
    }
}

impl<'a, T: 'static + ToOwned<Owned = T>> UseState<'a, T> {
    /// Gain mutable access to the new value via [`RefMut`].
    ///
    /// If `modify` is called, then the component will re-render.
    ///
    /// This method is only available when the value is a `ToOwned` type.
    ///
    /// Mutable access is derived by calling "ToOwned" (IE cloning) on the current value.
    ///
    /// To get a reference to the current value, use `.get()`
    pub fn modify(self) -> RefMut<'a, T> {
        // make sure we get processed
        self.needs_update();

        // Bring out the new value, cloning if it we need to
        // "get_mut" is locked behind "ToOwned" to make it explicit that cloning occurs to use this
        RefMut::map(self.inner.wip.borrow_mut(), |slot| {
            if slot.is_none() {
                *slot = Some(self.inner.current_val.as_ref().to_owned());
            }
            slot.as_mut().unwrap()
        })
    }

    pub fn inner(self) -> T {
        self.inner.current_val.as_ref().to_owned()
    }
}

impl<'a, T> std::ops::Deref for UseState<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.current_val)
    }
}
impl<'a, V, T: PartialEq<V>> PartialEq<V> for UseState<'a, T> {
    fn eq(&self, other: &V) -> bool {
        self.get() == other
    }
}
impl<'a, O, T: std::ops::Not<Output = O> + Copy> std::ops::Not for UseState<'a, T> {
    type Output = O;

    fn not(self) -> Self::Output {
        !*self.get()
    }
}

/*

Convenience methods for UseState.

Note!

This is not comprehensive.
This is *just* meant to make common operations easier.
*/

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

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

/// MUL
impl<'a, T: Copy + Mul<T, Output = T>> Mul<T> for UseState<'a, T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.inner.current_val.mul(rhs)
    }
}
impl<'a, T: Copy + Mul<T, Output = T>> MulAssign<T> for UseState<'a, T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set(self.inner.current_val.mul(rhs));
    }
}
/// DIV
impl<'a, T: Copy + Div<T, Output = T>> Div<T> for UseState<'a, T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.inner.current_val.div(rhs)
    }
}
impl<'a, T: Copy + Div<T, Output = T>> DivAssign<T> for UseState<'a, T> {
    fn div_assign(&mut self, rhs: T) {
        self.set(self.inner.current_val.div(rhs));
    }
}
