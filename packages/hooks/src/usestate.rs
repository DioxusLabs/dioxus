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
/// Usage:
///
/// ```ignore
/// const Example: Component = |cx| {
///     let counter = use_state(&cx, || 0);
///
///     cx.render(rsx! {
///         div {
///             h1 { "Counter: {counter}" }
///             button { onclick: move |_| counter.set(**counter + 1), "Increment" }
///             button { onclick: move |_| counter.set(**counter - 1), "Decrement" }
///         }
///     ))
/// }
/// ```
pub fn use_state<'a, T: 'static>(
    cx: &'a ScopeState,
    initial_state_fn: impl FnOnce() -> T,
) -> &'a UseState<T> {
    let hook = cx.use_hook(move |_| UseState {
        current_val: Rc::new(initial_state_fn()),
        update_callback: cx.schedule_update(),
        wip: Rc::new(RefCell::new(None)),
        update_scheuled: Cell::new(false),
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

    hook
}

pub struct UseState<T: 'static> {
    pub(crate) current_val: Rc<T>,
    pub(crate) wip: Rc<RefCell<Option<T>>>,
    pub(crate) update_callback: Rc<dyn Fn()>,
    pub(crate) update_scheuled: Cell<bool>,
}

impl<T: Debug> Debug for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.current_val)
    }
}

impl<T: 'static> UseState<T> {
    /// Tell the Dioxus Scheduler that we need to be processed
    pub fn needs_update(&self) {
        if !self.update_scheuled.get() {
            self.update_scheuled.set(true);
            (self.update_callback)();
        }
    }

    pub fn set(&self, new_val: T) {
        *self.wip.borrow_mut() = Some(new_val);
        self.needs_update();
    }

    pub fn get(&self) -> &T {
        &self.current_val
    }

    pub fn get_rc(&self) -> &Rc<T> {
        &self.current_val
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip(&self) -> Ref<Option<T>> {
        self.wip.borrow()
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip_mut(&self) -> RefMut<Option<T>> {
        self.wip.borrow_mut()
    }

    pub fn split(&self) -> (&T, Rc<dyn Fn(T)>) {
        (&self.current_val, self.setter())
    }

    pub fn setter(&self) -> Rc<dyn Fn(T)> {
        let slot = self.wip.clone();
        let callback = self.update_callback.clone();
        Rc::new(move |new| {
            callback();
            *slot.borrow_mut() = Some(new)
        })
    }

    pub fn wtih(&self, f: impl FnOnce(&mut T)) {
        let mut val = self.wip.borrow_mut();

        if let Some(inner) = val.as_mut() {
            f(inner);
        }
    }

    pub fn for_async(&self) -> UseState<T> {
        let UseState {
            current_val,
            wip,
            update_callback,
            update_scheuled,
        } = self;

        UseState {
            current_val: current_val.clone(),
            wip: wip.clone(),
            update_callback: update_callback.clone(),
            update_scheuled: update_scheuled.clone(),
        }
    }
}

impl<T: 'static + ToOwned<Owned = T>> UseState<T> {
    /// Gain mutable access to the new value via [`RefMut`].
    ///
    /// If `modify` is called, then the component will re-render.
    ///
    /// This method is only available when the value is a `ToOwned` type.
    ///
    /// Mutable access is derived by calling "ToOwned" (IE cloning) on the current value.
    ///
    /// To get a reference to the current value, use `.get()`
    pub fn modify(&self) -> RefMut<T> {
        // make sure we get processed
        self.needs_update();

        // Bring out the new value, cloning if it we need to
        // "get_mut" is locked behind "ToOwned" to make it explicit that cloning occurs to use this
        RefMut::map(self.wip.borrow_mut(), |slot| {
            if slot.is_none() {
                *slot = Some(self.current_val.as_ref().to_owned());
            }
            slot.as_mut().unwrap()
        })
    }

    pub fn inner(self) -> T {
        self.current_val.as_ref().to_owned()
    }
}

impl<'a, T> std::ops::Deref for UseState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.current_val)
    }
}

impl<'a, V, T: PartialEq<V>> PartialEq<V> for UseState<T> {
    fn eq(&self, other: &V) -> bool {
        self.get() == other
    }
}
impl<'a, O, T: std::ops::Not<Output = O> + Copy> std::ops::Not for UseState<T> {
    type Output = O;

    fn not(self) -> Self::Output {
        !*self.get()
    }
}
