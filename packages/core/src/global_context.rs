use std::sync::Arc;

use futures_util::Future;

use crate::{
    runtime::{with_current_scope, with_runtime},
    Element, ScopeId, Task,
};

/// Get the current scope id
pub fn current_scope_id() -> Option<ScopeId> {
    with_runtime(|rt| rt.current_scope_id()).flatten()
}

#[doc(hidden)]
/// Check if the virtual dom is currently inside of the body of a component
pub fn vdom_is_rendering() -> bool {
    with_runtime(|rt| rt.rendering.get()).unwrap_or_default()
}

/// Consume context from the current scope
pub fn consume_context<T: 'static + Clone>() -> Option<T> {
    with_current_scope(|cx| cx.consume_context::<T>()).flatten()
}

/// Consume context from the current scope
pub fn consume_context_from_scope<T: 'static + Clone>(scope_id: ScopeId) -> Option<T> {
    with_runtime(|rt| {
        rt.get_context(scope_id)
            .and_then(|cx| cx.consume_context::<T>())
    })
    .flatten()
}

/// Check if the current scope has a context
pub fn has_context<T: 'static + Clone>() -> Option<T> {
    with_current_scope(|cx| cx.has_context::<T>()).flatten()
}

/// Provide context to the current scope
pub fn provide_context<T: 'static + Clone>(value: T) -> T {
    with_current_scope(|cx| cx.provide_context(value)).expect("to be in a dioxus runtime")
}

/// Provide a context to the root scope
pub fn provide_root_context<T: 'static + Clone>(value: T) -> Option<T> {
    with_current_scope(|cx| cx.provide_root_context(value))
}

/// Suspends the current component
pub fn suspend() -> Option<Element> {
    with_current_scope(|cx| {
        cx.suspend();
    });
    None
}

/// Pushes the future onto the poll queue to be polled after the component renders.
pub fn push_future(fut: impl Future<Output = ()> + 'static) -> Option<Task> {
    with_current_scope(|cx| cx.push_future(fut))
}

/// Spawns the future but does not return the [`TaskId`]
pub fn spawn(fut: impl Future<Output = ()> + 'static) {
    with_current_scope(|cx| cx.spawn(fut));
}

/// Spawn a future that Dioxus won't clean up when this component is unmounted
///
/// This is good for tasks that need to be run after the component has been dropped.
pub fn spawn_forever(fut: impl Future<Output = ()> + 'static) -> Option<Task> {
    with_current_scope(|cx| cx.spawn_forever(fut))
}

/// Informs the scheduler that this task is no longer needed and should be removed.
///
/// This drops the task immediately.
pub fn remove_future(id: Task) {
    with_current_scope(|cx| cx.remove_future(id));
}

/// Store a value between renders. The foundational hook for all other hooks.
///
/// Accepts an `initializer` closure, which is run on the first use of the hook (typically the initial render). The return value of this closure is stored for the lifetime of the component, and a mutable reference to it is provided on every render as the return value of `use_hook`.
///
/// When the component is unmounted (removed from the UI), the value is dropped. This means you can return a custom type and provide cleanup code by implementing the [`Drop`] trait
///
/// # Example
///
/// ```
/// use dioxus_core::ScopeState;
///
/// // prints a greeting on the initial render
/// pub fn use_hello_world() {
///     once(|| println!("Hello, world!"));
/// }
/// ```
pub fn once<State: Clone + 'static>(initializer: impl FnOnce() -> State) -> State {
    with_current_scope(|cx| cx.use_hook(initializer)).expect("to be in a dioxus runtime")
}

pub fn use_hook<State: Clone + 'static>(initializer: impl FnOnce() -> State) -> State {
    once(initializer)
}

/// Get the current render since the inception of this component
///
/// This can be used as a helpful diagnostic when debugging hooks/renders, etc
pub fn generation() -> usize {
    with_current_scope(|cx| cx.generation()).expect("to be in a dioxus runtime")
}

/// Get the parent of the current scope if it exists
pub fn parent_scope() -> Option<ScopeId> {
    with_current_scope(|cx| cx.parent_id()).flatten()
}

/// Mark the current scope as dirty, causing it to re-render
pub fn needs_update() {
    with_current_scope(|cx| cx.needs_update());
}

/// Schedule an update for the current component
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
///
/// You should prefer [`schedule_update_any`] if you need to update multiple components.
pub fn schedule_update() -> Arc<dyn Fn() + Send + Sync> {
    with_current_scope(|cx| cx.schedule_update()).expect("to be in a dioxus runtime")
}

/// Schedule an update for any component given its [`ScopeId`].
///
/// A component's [`ScopeId`] can be obtained from the [`current_scope_id`] method.
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
pub fn schedule_update_any() -> Arc<dyn Fn(ScopeId) + Send + Sync> {
    with_current_scope(|cx| cx.schedule_update_any()).expect("to be in a dioxus runtime")
}
