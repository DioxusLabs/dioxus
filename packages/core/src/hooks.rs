//! Useful, foundational hooks that 3rd parties can implement.
//! Currently implemented:
//! - [x] use_ref
//! - [x] use_state
//! - [ ] use_reducer
//! - [ ] use_effect

use crate::innerlude::Context;

use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Store state between component renders!
/// When called, this hook retrives a stored value and provides a setter to update that value.
/// When the setter is called, the component is re-ran with the new value.
///
/// This is behaves almost exactly the same way as React's "use_state".
///
/// Usage:
/// ```ignore
/// static Example: FC<()> = |cx| {
///     let (counter, set_counter) = use_state(cx, || 0);
///     let increment = |_| set_couter(counter + 1);
///     let decrement = |_| set_couter(counter + 1);
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
pub fn use_state_classic<'a, 'c, T: 'static, F: FnOnce() -> T, P>(
    cx: Context<'a, P>,
    initial_state_fn: F,
) -> (&'a T, &'a Rc<dyn Fn(T)>) {
    struct UseState<T: 'static> {
        new_val: Rc<RefCell<Option<T>>>,
        current_val: T,
        caller: Rc<dyn Fn(T) + 'static>,
    }

    cx.use_hook(
        move || UseState {
            new_val: Rc::new(RefCell::new(None)),
            current_val: initial_state_fn(),
            caller: Rc::new(|_| println!("setter called!")),
        },
        move |hook| {
            log::debug!("Use_state set called");
            let inner = hook.new_val.clone();
            let scheduled_update = cx.schedule_update();

            // get ownership of the new val and replace the current with the new
            // -> as_ref -> borrow_mut -> deref_mut -> take
            // -> rc     -> &RefCell   -> RefMut    -> &Option<T> -> T
            if let Some(new_val) = hook.new_val.as_ref().borrow_mut().deref_mut().take() {
                hook.current_val = new_val;
            }

            // todo: swap out the caller with a subscription call and an internal update
            hook.caller = Rc::new(move |new_val| {
                // update the setter with the new value
                let mut new_inner = inner.as_ref().borrow_mut();
                *new_inner = Some(new_val);

                // Ensure the component gets updated
                scheduled_update();
            });

            // box gets derefed into a ref which is then taken as ref with the hook
            (&hook.current_val, &hook.caller)
        },
        |_| {},
    )
}

use crate::innerlude::*;
use std::{
    fmt::{Debug, Display},
    pin::Pin,
};

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
        (self.inner.callback)();
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

use std::ops::{Add, AddAssign};
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

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.current_val)
    }
}
struct UseStateInner<T: 'static> {
    current_val: T,
    callback: Rc<dyn Fn()>,
    wip: RefCell<Option<T>>,
}

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
        },
        move |hook| {
            log::debug!("addr of hook: {:#?}", hook as *const _);
            let mut new_val = hook.wip.borrow_mut();
            if new_val.is_some() {
                hook.current_val = new_val.take().unwrap();
            }

            UseState { inner: &*hook }
        },
        |_| {},
    )
}

/// Store a mutable value between renders!
/// To read the value, borrow the ref.
/// To change it, use modify.
/// Modifications to this value do not cause updates to the component
/// Attach to inner context reference, so context can be consumed
pub fn use_ref<'a, T: 'static, P>(
    cx: Context<'a, P>,
    initial_state_fn: impl FnOnce() -> T + 'static,
) -> &'a RefCell<T> {
    cx.use_hook(|| RefCell::new(initial_state_fn()), |state| &*state, |_| {})
}

struct UseReducer<T: 'static, R: 'static> {
    new_val: Rc<RefCell<Option<T>>>,
    current_val: T,
    caller: Box<dyn Fn(R) + 'static>,
}

/// Store state between component renders!
/// When called, this hook retrives a stored value and provides a setter to update that value.
/// When the setter is called, the component is re-ran with the new value.
///
/// This is behaves almost exactly the same way as React's "use_state".
///
pub fn use_reducer<'a, 'c, State: 'static, Action: 'static, P>(
    cx: Context<'a, P>,
    initial_state_fn: impl FnOnce() -> State,
    _reducer: impl Fn(&mut State, Action),
) -> (&'a State, &'a Box<dyn Fn(Action)>) {
    cx.use_hook(
        move || UseReducer {
            new_val: Rc::new(RefCell::new(None)),
            current_val: initial_state_fn(),
            caller: Box::new(|_| println!("setter called!")),
        },
        move |hook| {
            let _inner = hook.new_val.clone();
            let scheduled_update = cx.schedule_update();

            // get ownership of the new val and replace the current with the new
            // -> as_ref -> borrow_mut -> deref_mut -> take
            // -> rc     -> &RefCell   -> RefMut    -> &Option<T> -> T
            if let Some(new_val) = hook.new_val.as_ref().borrow_mut().deref_mut().take() {
                hook.current_val = new_val;
            }

            // todo: swap out the caller with a subscription call and an internal update
            hook.caller = Box::new(move |_new_val| {
                // update the setter with the new value
                // let mut new_inner = inner.as_ref().borrow_mut();
                // *new_inner = Some(new_val);

                // Ensure the component gets updated
                scheduled_update();
            });

            // box gets derefed into a ref which is then taken as ref with the hook
            (&hook.current_val, &hook.caller)
        },
        |_| {},
    )
}

/// Use model makes it easy to use "models" as state for components. To modify the model, call "modify" and a clone of the
/// current model will be made, with a RefMut lock on it. Dioxus will never run your components multithreaded, so you can
/// be relatively sure that this won't fail in practice
pub fn use_model<'a, T: ToOwned<Owned = T> + 'static, P>(
    cx: Context<'a, P>,
    f: impl FnOnce() -> T,
) -> &'a UseModel<T> {
    cx.use_hook(
        move || {
            let real = f();
            let wip = RefCell::new(real.to_owned());
            let update = cx.schedule_update();
            UseModel { real, wip, update }
        },
        |hook| {
            hook.real = hook.wip.borrow().to_owned();
            &*hook
        },
        |_| {},
    )
}

pub struct UseModel<T: ToOwned> {
    real: T,
    wip: RefCell<T>,
    update: Rc<dyn Fn()>,
}

use std::cell::{Ref, RefMut};
impl<T: ToOwned> Deref for UseModel<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.real
    }
}

impl<T: Display + ToOwned> Display for UseModel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.real)
    }
}

impl<T: ToOwned> UseModel<T> {
    pub fn get_mut(&self) -> RefMut<'_, T> {
        (self.update)();
        self.wip.borrow_mut()
    }
    pub fn modify(&self, f: impl FnOnce(&mut T)) {
        (self.update)();
        let mut g = self.get_mut();
        let r = g.deref_mut();
        f(r)
    }
}

// #[cfg(test)]
mod tests {

    use crate::prelude::*;

    enum Actions {
        Incr,
        Decr,
    }

    // #[allow(unused)]
    // static Example: FC<()> = |cx| {
    //     let (count, reduce) = use_reducer(
    //         &cx,
    //         || 0,
    //         |count, action| match action {
    //             Actions::Incr => *count += 1,
    //             Actions::Decr => *count -= 1,
    //         },
    //     );

    //     cx.render(rsx! {
    //         div {
    //             h1 {"Count: {count}"}
    //             button {
    //                 "Increment"
    //                 onclick: move |_| reduce(Actions::Incr)
    //             }
    //             button {
    //                 "Decrement"
    //                 onclick: move |_| reduce(Actions::Decr)
    //             }
    //         }
    //     })
    // };
}

pub fn use_is_initialized<'a, P>(cx: Context<'a, P>) -> bool {
    let val = use_ref(cx, || false);
    match *val.borrow() {
        true => true,
        false => {
            //
            *val.borrow_mut() = true;
            false
        }
    }
}
