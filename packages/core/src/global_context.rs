use crate::{runtime::Runtime, Element, ScopeId, Task};
use futures_util::Future;
use std::sync::Arc;

/// Get the current scope id
pub fn current_scope_id() -> Option<ScopeId> {
    Runtime::with(|rt| rt.current_scope_id()).flatten()
}

#[doc(hidden)]
/// Check if the virtual dom is currently inside of the body of a component
pub fn vdom_is_rendering() -> bool {
    Runtime::with(|rt| rt.rendering.get()).unwrap_or_default()
}

/// Consume context from the current scope
pub fn try_consume_context<T: 'static + Clone>() -> Option<T> {
    Runtime::with_current_scope(|cx| cx.consume_context::<T>()).flatten()
}

/// Consume context from the current scope
pub fn consume_context<T: 'static + Clone>() -> T {
    Runtime::with_current_scope(|cx| cx.consume_context::<T>())
        .flatten()
        .unwrap_or_else(|| panic!("Could not find context {}", std::any::type_name::<T>()))
}

/// Consume context from the current scope
pub fn consume_context_from_scope<T: 'static + Clone>(scope_id: ScopeId) -> Option<T> {
    Runtime::with(|rt| {
        rt.get_state(scope_id)
            .and_then(|cx| cx.consume_context::<T>())
    })
    .flatten()
}

/// Check if the current scope has a context
pub fn has_context<T: 'static + Clone>() -> Option<T> {
    Runtime::with_current_scope(|cx| cx.has_context::<T>()).flatten()
}

/// Provide context to the current scope
pub fn provide_context<T: 'static + Clone>(value: T) -> T {
    Runtime::with_current_scope(|cx| cx.provide_context(value)).expect("to be in a dioxus runtime")
}

/// Provide a context to the root scope
pub fn provide_root_context<T: 'static + Clone>(value: T) -> T {
    Runtime::with_current_scope(|cx| cx.provide_root_context(value))
        .expect("to be in a dioxus runtime")
}

/// Suspends the current component
pub fn suspend() -> Option<Element> {
    Runtime::with_current_scope(|cx| cx.suspend());
    None
}

/// Spawns the future but does not return the [`TaskId`]
pub fn spawn(fut: impl Future<Output = ()> + 'static) -> Task {
    Runtime::with_current_scope(|cx| cx.spawn(fut)).expect("to be in a dioxus runtime")
}

/// Spawn a future that Dioxus won't clean up when this component is unmounted
///
/// This is good for tasks that need to be run after the component has been dropped.
pub fn spawn_forever(fut: impl Future<Output = ()> + 'static) -> Option<Task> {
    Runtime::with_current_scope(|cx| cx.spawn_forever(fut))
}

/// Informs the scheduler that this task is no longer needed and should be removed.
///
/// This drops the task immediately.
pub fn remove_future(id: Task) {
    Runtime::with(|rt| rt.remove_task(id)).expect("Runtime to exist");
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
/// use dioxus_core::use_hook;
///
/// // prints a greeting on the initial render
/// pub fn use_hello_world() {
///     use_hook(|| println!("Hello, world!"));
/// }
/// ```
pub fn use_hook<State: Clone + 'static>(initializer: impl FnOnce() -> State) -> State {
    Runtime::with_current_scope(|cx| cx.use_hook(initializer)).expect("to be in a dioxus runtime")
}

/// Get the current render since the inception of this component
///
/// This can be used as a helpful diagnostic when debugging hooks/renders, etc
pub fn generation() -> usize {
    Runtime::with_current_scope(|cx| cx.generation()).expect("to be in a dioxus runtime")
}

/// Get the parent of the current scope if it exists
pub fn parent_scope() -> Option<ScopeId> {
    Runtime::with_current_scope(|cx| cx.parent_id()).flatten()
}

/// Mark the current scope as dirty, causing it to re-render
pub fn needs_update() {
    Runtime::with_current_scope(|cx| cx.needs_update());
}

/// Mark the current scope as dirty, causing it to re-render
pub fn needs_update_any(id: ScopeId) {
    Runtime::with_current_scope(|cx| cx.needs_update_any(id));
}

/// Schedule an update for the current component
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
///
/// You should prefer [`schedule_update_any`] if you need to update multiple components.
pub fn schedule_update() -> Arc<dyn Fn() + Send + Sync> {
    Runtime::with_current_scope(|cx| cx.schedule_update()).expect("to be in a dioxus runtime")
}

/// Schedule an update for any component given its [`ScopeId`].
///
/// A component's [`ScopeId`] can be obtained from the [`current_scope_id`] method.
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
pub fn schedule_update_any() -> Arc<dyn Fn(ScopeId) + Send + Sync> {
    Runtime::with_current_scope(|cx| cx.schedule_update_any()).expect("to be in a dioxus runtime")
}

/// Creates a callback that will be run before the component is removed.
/// This can be used to clean up side effects from the component
/// (created with [`use_effect`](crate::use_effect)).
///
/// Example:
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn app() -> Element {
///     let state = use_signal(|| true);
///     rsx! {
///         for _ in 0..100 {
///             h1 {
///                 "spacer"
///             }
///         }
///         if **state {
///             rsx! {
///                 child_component {}
///             }
///         }
///         button {
///             onclick: move |_| {
///                 state.set(!*state.get());
///             },
///             "Unmount element"
///         }
///     }
/// }
///
/// fn child_component() -> Element {
///     let original_scroll_position = use_signal(|| 0.0);
///     use_effect((), move |_| {
///         to_owned![original_scroll_position];
///         async move {
///             let window = web_sys::window().unwrap();
///             let document = window.document().unwrap();
///             let element = document.get_element_by_id("my_element").unwrap();
///             element.scroll_into_view();
///             original_scroll_position.set(window.scroll_y().unwrap());
///         }
///     });
///
///     use_drop({
///         to_owned![original_scroll_position];
///         /// restore scroll to the top of the page
///         move || {
///             let window = web_sys::window().unwrap();
///             window.scroll_with_x_and_y(*original_scroll_position.current(), 0.0);
///         }
///     });
///
///     rsx!{
///         div {
///             id: "my_element",
///             "hello"
///         }
///     }
/// }
/// ```
pub fn use_drop<D: FnOnce() + 'static>(destroy: D) {
    struct LifeCycle<D: FnOnce()> {
        /// Wrap the closure in an option so that we can take it out on drop.
        ondestroy: Option<D>,
    }

    /// On drop, we want to run the closure.
    impl<D: FnOnce()> Drop for LifeCycle<D> {
        fn drop(&mut self) {
            if let Some(f) = self.ondestroy.take() {
                f();
            }
        }
    }

    // We need to impl clone for the lifecycle, but we don't want the drop handler for the closure to be called twice.
    impl<D: FnOnce()> Clone for LifeCycle<D> {
        fn clone(&self) -> Self {
            Self { ondestroy: None }
        }
    }

    use_hook(|| LifeCycle {
        ondestroy: Some(destroy),
    });
}

/// A hook that allows you to insert a "before render" function.
///
/// This function will always be called before dioxus tries to render your component. This should be used for safely handling
/// early returns
pub fn use_before_render(f: impl FnMut() + 'static) {
    use_hook(|| before_render(f));
}

/// Push this function to be run after the next render
///
/// This function will always be called before dioxus tries to render your component. This should be used for safely handling
/// early returns
pub fn use_after_render(f: impl FnMut() + 'static) {
    use_hook(|| after_render(f));
}

/// Push a function to be run before the next render
/// This is a hook and will always run, so you can't unschedule it
/// Will run for every progression of suspense, though this might change in the future
pub fn before_render(f: impl FnMut() + 'static) {
    Runtime::with_current_scope(|cx| cx.push_before_render(f));
}

/// Push a function to be run after the render is complete, even if it didn't complete successfully
pub fn after_render(f: impl FnMut() + 'static) {
    Runtime::with_current_scope(|cx| cx.push_after_render(f));
}

/// Wait for the virtualdom to finish its sync work before proceeding
///
/// This is useful if you've just triggered an update and want to wait for it to finish before proceeding with valid
/// DOM nodes.
///
/// Effects rely on this to ensure that they only run effects after the DOM has been updated. Without flush_sync effects
/// are run immediately before diffing the DOM, which causes all sorts of out-of-sync weirdness.
pub async fn flush_sync() {
    // Wait for the flush lock to be available
    // We release it immediately, so it's impossible for the lock to be held longer than this function
    Runtime::with(|rt| rt.flush_mutex.clone())
        .unwrap()
        .lock()
        .await;
}

/// Use a hook with a cleanup function
pub fn use_hook_with_cleanup<T: Clone + 'static>(
    hook: impl FnOnce() -> T,
    cleanup: impl FnOnce(T) + 'static,
) -> T {
    let value = use_hook(hook);
    let _value = value.clone();
    use_drop(move || cleanup(_value));
    value
}
