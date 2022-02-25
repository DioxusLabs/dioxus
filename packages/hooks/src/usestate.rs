#![warn(clippy::pedantic)]

use dioxus_core::prelude::*;
use std::{
    cell::{RefCell, RefMut},
    fmt::{Debug, Display},
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
///     let (count, set_count) = use_state(&cx, || 0);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Count: {count}" }
///             button { onclick: move |_| set_count(a - 1), "Increment" }
///             button { onclick: move |_| set_count(a + 1), "Decrement" }
///         }
///     ))
/// }
/// ```
pub fn use_state<'a, T: 'static>(
    cx: &'a ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> (&'a T, &'a UseState<T>) {
    let hook = cx.use_hook(move |_| {
        let current_val = Rc::new(initial_state_fn());
        let update_callback = cx.schedule_update();
        let slot = Rc::new(RefCell::new(current_val.clone()));
        let setter = Rc::new({
            crate::to_owned![update_callback, slot];
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

    (hook.current_val.as_ref(), hook)
}

pub struct UseState<T: 'static> {
    pub(crate) current_val: Rc<T>,
    pub(crate) update_callback: Arc<dyn Fn()>,
    pub(crate) setter: Rc<dyn Fn(T)>,
    pub(crate) slot: Rc<RefCell<Rc<T>>>,
}

impl<T: 'static> UseState<T> {
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
    ///     let (count, set_count) = use_state(&cx, || 0);
    ///     cx.spawn({
    ///         let set_count = set_count.to_owned();
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
    /// However, for most cases, calling `to_owned` o`UseState`te is the
    /// preferred way to get "anoth`set_state`tate handle.
    ///
    ///
    /// # Examples
    /// A component might require an `Rc<dyn Fn(T)>` as an input to set a value.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let (value, set_value) = use_state(&cx, || 0);
    ///
    ///     rsx!{
    ///         Component {
    ///             handler: set_val.setter()
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
    /// ```rust
    /// # use dioxus_core::prelude::*;
    /// # use dioxus_hooks::*;
    /// fn component(cx: Scope) -> Element {
    ///     let (value, set_value) = use_state(&cx, || 0);
    ///
    ///     // to increment the value
    ///     set_value.modify(|v| v + 1);
    ///
    ///     // usage in async
    ///     cx.spawn({
    ///         let set_value = set_value.to_owned();
    ///         async move {
    ///             set_value.modify(|v| v + 1);
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
    ///     let (value, set_value) = use_state(&cx, || 0);
    ///
    ///     let as_rc = set_value.get();
    ///     assert_eq!(as_rc.as_ref(), &0);
    ///
    ///     # todo!()
    /// }
    /// ```
    #[must_use]
    pub fn get(&self) -> &Rc<T> {
        &self.current_val
    }

    /// Mark the component that create this [`UseState`] as dirty, forcing it to re-render.
    ///
    /// ```rust, ignore
    /// fn component(cx: Scope) -> Element {
    ///     let (count, set_count) = use_state(&cx, || 0);
    ///     cx.spawn({
    ///         let set_count = set_count.to_owned();
    ///         async move {
    ///             // for the component to re-render
    ///             set_count.needs_update();
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
    /// ```
    /// let (val, set_val) = use_state(&cx, || 0);
    ///
    /// set_val.with_mut(|v| *v = 1);
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
    /// ```
    /// let (val, set_val) = use_state(&cx, || 0);
    ///
    /// set_val.with_mut(|v| *v = 1);
    /// ```
    #[must_use]
    pub fn make_mut(&self) -> RefMut<T> {
        let mut slot = self.slot.borrow_mut();

        self.needs_update();

        if Rc::strong_count(&*slot) > 0 {
            *slot = Rc::new(slot.as_ref().to_owned());
        }

        RefMut::map(slot, |rc| Rc::get_mut(rc).expect("the hard count to be 0"))
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

impl<'a, T: 'static + Display> std::fmt::Display for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.current_val)
    }
}

impl<T> PartialEq<UseState<T>> for UseState<T> {
    fn eq(&self, other: &UseState<T>) -> bool {
        // some level of memoization for UseState
        Rc::ptr_eq(&self.slot, &other.slot)
    }
}

impl<T: Debug> Debug for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.current_val)
    }
}

impl<'a, T> std::ops::Deref for UseState<T> {
    type Target = Rc<dyn Fn(T)>;

    fn deref(&self) -> &Self::Target {
        &self.setter
    }
}

#[test]
fn api_makes_sense() {
    #[allow(unused)]
    fn app(cx: Scope) -> Element {
        let (val, set_val) = use_state(&cx, || 0);

        set_val(0);
        set_val.modify(|v| v + 1);
        let real_current = set_val.current();

        match val {
            10 => {
                set_val(20);
                set_val.modify(|v| v + 1);
            }
            20 => {}
            _ => {
                println!("{real_current}");
            }
        }

        cx.spawn({
            crate::to_owned![set_val];
            async move {
                set_val.modify(|f| f + 1);
            }
        });

        cx.render(LazyNodes::new(|f| f.static_text("asd")))
    }
}
