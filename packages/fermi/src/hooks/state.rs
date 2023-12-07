use crate::{AtomId, AtomRoot, Writable};
use dioxus_core::{ScopeId, ScopeState};
use std::{
    cell::RefMut,
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Not, Sub},
    rc::Rc,
};

/// Store state between component renders.
///
/// ## Dioxus equivalent of AtomState, designed for Rust
///
/// The Dioxus version of `AtomState` for state management inside components. It allows you to ergonomically store and
/// modify state between component renders. When the state is updated, the component will re-render.
///
///
/// ```ignore
/// static COUNT: Atom<u32> = |_| 0;
///
/// fn Example(cx: Scope) -> Element {
///     let mut count = use_atom_state(cx, &COUNT);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Count: {count}" }
///             button { onclick: move |_| count += 1, "Increment" }
///             button { onclick: move |_| count -= 1, "Decrement" }
///         }
///     ))
/// }
/// ```
#[must_use]
pub fn use_atom_state<T: 'static>(cx: &ScopeState, f: impl Writable<T>) -> &AtomState<T> {
    let root = crate::use_atom_root(cx);

    let inner = cx.use_hook(|| AtomState {
        value: None,
        root: root.clone(),
        scope_id: cx.scope_id(),
        id: f.unique_id(),
    });

    inner.value = Some(inner.root.register(f, cx.scope_id()));

    inner
}

pub struct AtomState<V: 'static> {
    root: Rc<AtomRoot>,
    id: AtomId,
    scope_id: ScopeId,
    value: Option<Rc<V>>,
}

impl<V> Drop for AtomState<V> {
    fn drop(&mut self) {
        self.root.unsubscribe(self.id, self.scope_id)
    }
}

impl<T: 'static> AtomState<T> {
    /// Set the state to a new value.
    pub fn set(&self, new: T) {
        self.root.set(self.id, new)
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
        let atoms = self.root.atoms.borrow();
        let slot = atoms.get(&self.id).unwrap();
        slot.value.clone().downcast().unwrap()
    }

    /// Get the `setter` function directly without the `AtomState` wrapper.
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
        let root = self.root.clone();
        let id = self.id;
        Rc::new(move |new_val| root.set(id, new_val))
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
        self.root.clone().set(self.id, {
            let current = self.value.as_ref().unwrap();
            f(current.as_ref())
        });
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
        self.value.as_ref().unwrap()
    }

    #[must_use]
    pub fn get_rc(&self) -> &Rc<T> {
        self.value.as_ref().unwrap()
    }

    /// Mark all consumers of this atom to re-render
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
        self.root.force_update(self.id)
    }
}

impl<T: Clone> AtomState<T> {
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
    /// ```ignore
    /// let val = use_state(cx, || 0);
    ///
    /// val.with_mut(|v| *v = 1);
    /// ```
    pub fn with_mut(&self, apply: impl FnOnce(&mut T)) {
        let mut new_val = self.value.as_ref().unwrap().as_ref().to_owned();
        apply(&mut new_val);
        self.set(new_val);
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
    /// ```ignore
    /// let val = use_state(cx, || 0);
    ///
    /// *val.make_mut() += 1;
    /// ```
    #[must_use]
    pub fn make_mut(&self) -> RefMut<T> {
        todo!("make mut not support for atom values yet")
        // let mut slot = self.value.as_ref().unwrap();

        // self.needs_update();

        // if Rc::strong_count(&*slot) > 0 {
        //     *slot = Rc::new(slot.as_ref().to_owned());
        // }

        // RefMut::map(slot, |rc| Rc::get_mut(rc).expect("the hard count to be 0"))
    }

    /// Convert this handle to a tuple of the value and the handle itself.
    #[must_use]
    pub fn split(&self) -> (&T, &Self) {
        (self.value.as_ref().unwrap(), self)
    }
}

impl<T: 'static> Clone for AtomState<T> {
    fn clone(&self) -> Self {
        AtomState {
            root: self.root.clone(),
            id: self.id,
            scope_id: self.scope_id,
            value: self.value.clone(),
        }
    }
}

impl<T: 'static + Display> std::fmt::Display for AtomState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.as_ref().unwrap())
    }
}

impl<T: std::fmt::Binary> std::fmt::Binary for AtomState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.value.as_ref().unwrap().as_ref())
    }
}

impl<T: PartialEq> PartialEq<T> for AtomState<T> {
    fn eq(&self, other: &T) -> bool {
        self.value.as_ref().unwrap().as_ref() == other
    }
}

// todo: this but for more interesting conrete types
impl PartialEq<bool> for &AtomState<bool> {
    fn eq(&self, other: &bool) -> bool {
        self.value.as_ref().unwrap().as_ref() == other
    }
}

impl<T: PartialEq> PartialEq<AtomState<T>> for AtomState<T> {
    fn eq(&self, other: &AtomState<T>) -> bool {
        Rc::ptr_eq(self.value.as_ref().unwrap(), other.value.as_ref().unwrap())
    }
}

impl<T: Debug> Debug for AtomState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value.as_ref().unwrap())
    }
}

impl<T> std::ops::Deref for AtomState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap().as_ref()
    }
}

impl<T: Not + Copy> std::ops::Not for &AtomState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value.as_ref().unwrap().not()
    }
}

impl<T: Not + Copy> std::ops::Not for AtomState<T> {
    type Output = <T as std::ops::Not>::Output;

    fn not(self) -> Self::Output {
        self.value.as_ref().unwrap().not()
    }
}

impl<T: std::ops::Add + Copy> std::ops::Add<T> for &AtomState<T> {
    type Output = <T as std::ops::Add>::Output;

    fn add(self, other: T) -> Self::Output {
        *self.value.as_ref().unwrap().as_ref() + other
    }
}
impl<T: std::ops::Sub + Copy> std::ops::Sub<T> for &AtomState<T> {
    type Output = <T as std::ops::Sub>::Output;

    fn sub(self, other: T) -> Self::Output {
        *self.value.as_ref().unwrap().as_ref() - other
    }
}

impl<T: std::ops::Div + Copy> std::ops::Div<T> for &AtomState<T> {
    type Output = <T as std::ops::Div>::Output;

    fn div(self, other: T) -> Self::Output {
        *self.value.as_ref().unwrap().as_ref() / other
    }
}

impl<T: std::ops::Mul + Copy> std::ops::Mul<T> for &AtomState<T> {
    type Output = <T as std::ops::Mul>::Output;

    fn mul(self, other: T) -> Self::Output {
        *self.value.as_ref().unwrap().as_ref() * other
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for &AtomState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for &AtomState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for &AtomState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for &AtomState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}

impl<T: Add<Output = T> + Copy> std::ops::AddAssign<T> for AtomState<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set((*self.current()) + rhs);
    }
}

impl<T: Sub<Output = T> + Copy> std::ops::SubAssign<T> for AtomState<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set((*self.current()) - rhs);
    }
}

impl<T: Mul<Output = T> + Copy> std::ops::MulAssign<T> for AtomState<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set((*self.current()) * rhs);
    }
}

impl<T: Div<Output = T> + Copy> std::ops::DivAssign<T> for AtomState<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set((*self.current()) / rhs);
    }
}
