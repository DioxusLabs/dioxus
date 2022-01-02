mod handle;
mod owned;
pub use handle::*;
pub use owned::*;

use dioxus_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
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
    let hook = cx.use_hook(move |_| UseStateOwned {
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

    UseState(hook)
}
