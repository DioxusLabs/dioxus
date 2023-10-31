#![warn(clippy::pedantic)]

use dioxus_core::prelude::*;
use std::{
    cell::{RefCell, RefMut},
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Not, Sub},
    rc::Rc,
    sync::Arc,
};

/// Store state between component renders.
///
/// ## Dioxus equivalent of useState, designed for Rust
///
/// The Dioxus version of `useState` for state management inside components. It allows you to ergonomically store and
/// modify state between component renders. When the state is updated, the component will re-render.
///
///
/// ```ignore
/// const Example: Component = |cx| {
///     let count = use_state(cx, || 0);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Count: {count}" }
///             button { onclick: move |_| *count.modify() += 1, "Increment" }
///             button { onclick: move |_| *count.modify() -= 1, "Decrement" }
///         }
///     ))
/// }
/// ```
#[must_use]
pub fn use_state<T: 'static>(
    cx: &ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> &UseState<T> {
    let hook = cx.use_hook(move || {
        let current_val = Rc::new(initial_state_fn());
        let update_callback = cx.schedule_update();
        let slot = Rc::new(RefCell::new(current_val.clone()));
        let setter = Rc::new({
            to_owned![update_callback, slot];
            move |new| {
                {
                    let mut slot = slot.borrow_mut();

                    // if there's only one reference (weak or otherwise), we can just swap the values
                    // Typically happens when the state is set multiple times - we don't want to create a new Rc for each new value
                    if let Some(val) = Rc::get_mut(&mut slot) {
                        *val = new;
                    } else {
                        *slot = Rc::new(new);
                    }
                }
                update_callback();
            }
        });

        UseState {
            current_val,
            update_callback,
            setter,
            slot,
        }
    });

    hook.current_val = hook.slot.borrow().clone();

    hook
}

pub struct UseState<T: 'static> {
    pub(crate) current_val: Rc<T>,
    pub(crate) update_callback: Arc<dyn Fn()>,
    pub(crate) setter: Rc<dyn Fn(T)>,
    pub(crate) slot: Rc<RefCell<Rc<T>>>,
}

impl<T: 'static> UseState<T> {
    /// Set the state to a new value.
    pub fn set(&self, new: T) {
        (self.setter)(new);
    }

    /// Get the current value of the state by cloning its container Rc.
    ///
    /// This is useful when you are dealing with state in async contexts but need
    /// to know the current value. You are not given a reference to the state.
    ///
    /// # Examples
    /// An async context might need to know the current value:
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let count = use_state(cx, || 0);
    ///     cx.spawn({
    ///         let set_count = count.to_owned();
    ///         async move {
    ///             let current = set_count.current();
    ///         }
    ///     })
    /// }
    /// ```
    #[must_use]
    pub fn current(&self) -> Rc<T> {
        self.slot.borrow().clone()
    }

    /// Get the `setter` function directly without the `UseState` wrapper.
    ///
    /// This is useful for passing the setter function to other components.
    ///
    /// However, for most cases, calling `to_owned` on the state is the
    /// preferred way to get "another" state handle.
    ///
    ///
    /// # Examples
    /// A component might require an `Rc<dyn Fn(T)>` as an input to set a value.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(cx, || 0);
    ///
    ///     rsx!{
    ///         Component {
    ///             handler: value.setter()
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn setter(&self) -> Rc<dyn Fn(T)> {
        self.setter.clone()
    }

    /// Set the state to a new value, using the current state value as a reference.
    ///
    /// This is similar to passing a closure to React's `set_value` function.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```rust, ignore
    /// # use dioxus_core::prelude::*;
    /// # use dioxus_hooks::*;
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(cx, || 0);
    ///
    ///     // to increment the value
    ///     value.modify(|v| v + 1);
    ///
    ///     // usage in async
    ///     cx.spawn({
    ///         let value = value.to_owned();
    ///         async move {
    ///             value.modify(|v| v + 1);
    ///         }
    ///     });
    ///
    ///     # todo!()
    /// }
    /// ```
    pub fn modify(&self, f: impl FnOnce(&T) -> T) {
        let new_val = {
            let current = self.slot.borrow();
            f(current.as_ref())
        };
        (self.setter)(new_val);
    }

    /// Get the value of the state when this handle was created.
    ///
    /// This method is useful when you want an `Rc` around the data to cheaply
    /// pass it around your app.
    ///
    /// ## Warning
    ///
    /// This will return a stale value if used within async contexts.
    ///
    /// Try `current` to get the real current value of the state.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// # use dioxus_core::prelude::*;
    /// # use dioxus_hooks::*;
    /// fn component(cx: Scope) -> Element {
    ///     let value = use_state(cx, || 0);
    ///
    ///     let as_rc = value.get();
    ///     assert_eq!(as_rc.as_ref(), &0);
    ///
    ///     # todo!()
    /// }
    /// ```
    #[must_use]
    pub fn get(&self) -> &T {
        &self.current_val
    }

    #[must_use]
    pub fn get_rc(&self) -> &Rc<T> {
        &self.current_val
    }

    /// Mark the component that create this [`UseState`] as dirty, forcing it to re-render.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let count = use_state(cx, || 0);
    ///     cx.spawn({
    ///         let count = count.to_owned();
    ///         async move {
    ///             // for the component to re-render
    ///             count.needs_update();
    ///         }
    ///     })
    /// }
    /// ```
    pub fn needs_update(&self) {
        (self.update_callback)();
    }
}

impl<T: Clone> UseState<T> {
    /// Get a mutable handle to the value by calling `ToOwned::to_owned` on the
    /// current value.
    ///
    /// This is essentially cloning the underlying value and then setting it,
    /// giving you a mutable handle in the process. This method is intended for
    /// types that are cheaply cloneable.
    ///
    /// If you are comfortable dealing with `RefMut`, then you can use `make_mut` to get
    /// the underlying slot. However, be careful with `RefMut` since you might panic
    /// if the `RefCell` is left open.
    ///
    /// # Examples
    ///
    /// ```rust, ignore
    /// let val = use_state(cx, || 0);
    ///
    /// val.with_mut(|v| *v = 1);
    /// ```
    pub fn with_mut(&self, apply: impl FnOnce(&mut T)) {
        let mut slot = self.slot.borrow_mut();
        let mut inner = slot.as_ref().to_owned();

        apply(&mut inner);

        if let Some(new) = Rc::get_mut(&mut slot) {
            *new = inner;
        } else {
            *slot = Rc::new(inner);
        }

        self.needs_update();
    }

    /// Get a mutable handle to the value by calling `ToOwned::to_owned` on the
    /// current value.
    ///
    /// This is essentially cloning the underlying value and then setting it,
    /// giving you a mutable handle in the process. This method is intended for
    /// types that are cheaply cloneable.
    ///
    /// # Warning
    /// Be careful with `RefMut` since you might panic if the `RefCell` is left open!
    ///
    /// # Examples
    ///
    /// ```rust, ignore
    /// let val = use_state(cx, || 0);
    ///
    /// *val.make_mut() += 1;
    /// ```
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn make_mut(&self) -> RefMut<T> {
        let mut slot = self.slot.borrow_mut();

        self.needs_update();

        if Rc::strong_count(&*slot) > 0 {
            *slot = Rc::new(slot.as_ref().to_owned());
        }

        RefMut::map(slot, |rc| Rc::get_mut(rc).expect("the hard count to be 0"))
    }

    /// Convert this handle to a tuple of the value and the handle itself.
    #[must_use]
    pub fn split(&self) -> (&T, &Self) {
        (&self.current_val, self)
    }
}

impl<T: 'static> Clone for UseState<T> {
    fn clone(&self) -> Self {
        UseState {
            current_val: self.current_val.clone(),
            update_callback: self.update_callback.clone(),
            setter: self.setter.clone(),
            slot: self.slot.clone(),
        }
    }
}

impl<T: 'static + Display> std::fmt::Display for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.current_val)
    }
}

impl<T: std::fmt::Binary> std::fmt::Binary for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.current_val.as_ref())
    }
}

impl<T: PartialEq> PartialEq<T> for UseState<T> {
    fn eq(&self, other: &T) -> bool {
        self.current_val.as_ref() == other
    }
}

// todo: this but for more interesting conrete types
impl PartialEq<bool> for &UseState<bool> {
    fn eq(&self, other: &bool) -> bool {
        self.current_val.as_ref() == other
    }
}

impl<T> PartialEq<UseState<T>> for UseState<T> {
    fn eq(&self, other: &UseState<T>) -> bool {
        Rc::ptr_eq(&self.current_val, &other.current_val)
    }
}

impl<T: std::cmp::PartialOrd> PartialOrd<T> for UseState<T> {
    fn ge(&self, other: &T) -> bool {
        *self.current_val >= *other
    }

    fn gt(&self, other: &T) -> bool {
        *self.current_val > *other
    }

    fn le(&self, other: &T) -> bool {
        *self.current_val <= *other
    }

    fn lt(&self, other: &T) -> bool {
        *self.current_val < *other
    }

    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        (*self.current_val).partial_cmp(other)
    }
}

impl<T: std::cmp::PartialOrd> PartialOrd<UseState<T>> for UseState<T> {
    fn ge(&self, other: &UseState<T>) -> bool {
        self.current_val >= other.current_val
    }

    fn gt(&self, other: &UseState<T>) -> bool {
        self.current_val > other.current_val
    }

    fn le(&self, other: &UseState<T>) -> bool {
        self.current_val <= other.current_val
    }

    fn lt(&self, other: &UseState<T>) -> bool {
        self.current_val < other.current_val
    }

    fn partial_cmp(&self, other: &UseState<T>) -> Option<std::cmp::Ordering> {
        self.current_val.partial_cmp(&other.current_val)
    }
}

impl<T: Debug> Debug for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.current_val)
    }
}

impl<T> std::ops::Deref for UseState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.current_val.as_ref()
    }
}

impl<T: Not + Copy> std::ops::Not for &UseState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.current_val.not()
    }
}

impl<T: Not + Copy> std::ops::Not for UseState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.current_val.not()
    }
}

impl<T: std::ops::Add + Copy> std::ops::Add<T> for &UseState<T> {
    type Output = <T as std::ops::Add>::Output;

    fn add(self, other: T) -> Self::Output {
        *self.current_val.as_ref() + other
    }
}
impl<T: std::ops::Sub + Copy> std::ops::Sub<T> for &UseState<T> {
    type Output = <T as std::ops::Sub>::Output;

    fn sub(self, other: T) -> Self::Output {
        *self.current_val.as_ref() - other
    }
}

impl<T: std::ops::Div + Copy> std::ops::Div<T> for &UseState<T> {
    type Output = <T as std::ops::Div>::Output;

    fn div(self, other: T) -> Self::Output {
        *self.current_val.as_ref() / other
    }
}

impl<T: std::ops::Mul + Copy> std::ops::Mul<T> for &UseState<T> {
    type Output = <T as std::ops::Mul>::Output;

    fn mul(self, other: T) -> Self::Output {
        *self.current_val.as_ref() * other
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for &UseState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for &UseState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for &UseState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for &UseState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for UseState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for UseState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for UseState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for UseState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}

#[test]
fn api_makes_sense() {
    #[allow(unused)]
    fn app(cx: Scope) -> Element {
        let val = use_state(cx, || 0);

        val.set(0);
        val.modify(|v| v + 1);
        let real_current = val.current();

        match val.get() {
            10 => {
                val.set(20);
                val.modify(|v| v + 1);
            }
            20 => {}
            _ => {
                println!("{real_current}");
            }
        }

        cx.spawn({
            to_owned![val];
            async move {
                val.modify(|f| f + 1);
            }
        });

        // cx.render(LazyNodes::new(|f| f.static_text("asd")))

        todo!()
    }
}
