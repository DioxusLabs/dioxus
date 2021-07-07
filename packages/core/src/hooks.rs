//! Useful, foundational hooks that 3rd parties can implement.
//! Currently implemented:
//! - [x] use_ref
//! - [x] use_state
//! - [ ] use_reducer
//! - [ ] use_effect

use crate::innerlude::Context;

use crate::innerlude::*;
use std::{
    cell::RefCell,
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
///     let (counter, set_counter) = use_state(&cx, || 0);
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
pub fn use_state_classic<'a, 'c, T: 'static, F: FnOnce() -> T>(
    cx: impl Scoped<'a>,
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

pub struct UseState<T: 'static> {
    modifier: Rc<RefCell<Option<Box<dyn FnOnce(&mut T)>>>>,
    current_val: T,
    update: Box<dyn Fn() + 'static>,
    setter: Box<dyn Fn(T) + 'static>,
    // setter: Box<dyn Fn(T) + 'static>,
}

// #[derive(Clone, Copy)]
// pub struct UseStateHandle<'a, T: 'static> {
//     inner: &'a UseState<T>,
//     // pub setter: &'a dyn Fn(T),
//     // pub modifier: &'a dyn Fn(&mut T),
// }

impl<'a, T: 'static> UseState<T> {
    pub fn setter(&self) -> &dyn Fn(T) {
        &self.setter
        // let r = self.setter.as_mut();
        // unsafe { Pin::get_unchecked_mut(r) }
    }

    pub fn set(&self, new_val: T) {
        self.modify(|f| *f = new_val);
    }

    // signal that we need to be updated
    // save the modifier
    pub fn modify(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut slot = self.modifier.as_ref().borrow_mut();
        *slot = Some(Box::new(f));
        (self.update)();
    }
}
impl<'a, T: 'static> std::ops::Deref for UseState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current_val
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.current_val)
    }
}

/// Store state between component renders!
/// When called, this hook retrives a stored value and provides a setter to update that value.
/// When the setter is called, the component is re-ran with the new value.
///
/// This is behaves almost exactly the same way as React's "use_state".
///
/// Usage:
/// ```ignore
/// static Example: FC<()> = |cx| {
///     let (counter, set_counter) = use_state(&cx, || 0);
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
pub fn use_state<'a, 'c, T: 'static, F: FnOnce() -> T>(
    cx: impl Scoped<'a>,
    initial_state_fn: F,
) -> &'a UseState<T> {
    cx.use_hook(
        move || UseState {
            modifier: Rc::new(RefCell::new(None)),
            current_val: initial_state_fn(),
            update: Box::new(|| {}),
            setter: Box::new(|_| {}),
        },
        move |hook| {
            log::debug!("addr of hook: {:#?}", hook as *const _);
            let scheduled_update = cx.schedule_update();

            // log::debug!("Checking if new value {:#?}", &hook.current_val);
            // get ownership of the new val and replace the current with the new
            // -> as_ref -> borrow_mut -> deref_mut -> take
            // -> rc     -> &RefCell   -> RefMut    -> &Option<T> -> T
            if let Some(new_val) = hook.modifier.as_ref().borrow_mut().deref_mut().take() {
                // log::debug!("setting prev {:#?}", &hook.current_val);
                (new_val)(&mut hook.current_val);
                // log::debug!("setting new value {:#?}", &hook.current_val);
            }

            hook.update = Box::new(move || scheduled_update());

            let modifier = hook.modifier.clone();
            hook.setter = Box::new(move |new_val: T| {
                let mut slot = modifier.as_ref().borrow_mut();

                let slot2 = slot.deref_mut();
                *slot2 = Some(Box::new(move |old: &mut T| *old = new_val));
            });

            &*hook
        },
        |_| {},
    )
}

// pub struct UseRef<T: 'static> {
//     _current: RefCell<T>,
// }
// impl<T: 'static> UseRef<T> {
//     fn new(val: T) -> Self {
//         Self {
//             _current: RefCell::new(val),
//         }
//     }

//     pub fn set(&self, new: T) {
//         let mut val = self._current.borrow_mut();
//         *val = new;
//     }

//     pub fn modify(&self, modifier: impl FnOnce(&mut T)) {
//         let mut val = self._current.borrow_mut();
//         let val_as_ref = val.deref_mut();
//         modifier(val_as_ref);
//     }

//     pub fn current(&self) -> std::cell::Ref<'_, T> {
//         self._current.borrow()
//     }
// }

/// Store a mutable value between renders!
/// To read the value, borrow the ref.
/// To change it, use modify.
/// Modifications to this value do not cause updates to the component
/// Attach to inner context reference, so context can be consumed
pub fn use_ref<'a, T: 'static>(
    cx: impl Scoped<'a>,
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
pub fn use_reducer<'a, 'c, State: 'static, Action: 'static>(
    cx: impl Scoped<'a>,
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
pub fn use_model<'a, T: ToOwned<Owned = T> + 'static>(
    cx: impl Scoped<'a>,
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

pub fn use_is_initialized<P>(cx: Context<P>) -> bool {
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
