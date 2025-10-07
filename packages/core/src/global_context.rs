use crate::innerlude::CapturedError;
use crate::{innerlude::SuspendedFuture, runtime::Runtime, Element, ScopeId, Task};
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

/// Get the current scope id
pub fn current_scope_id() -> ScopeId {
    Runtime::with(|rt| rt.current_scope_id())
}

/// Throw a [`CapturedError`] into the current scope. The error will bubble up to the nearest [`crate::ErrorBoundary()`] or the root of the app.
///
/// # Examples
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn Component() -> Element {
///     let request = spawn(async move {
///         match reqwest::get("https://api.example.com").await {
///             Ok(_) => unimplemented!(),
///             // You can explicitly throw an error into a scope with throw_error
///             Err(err) => dioxus::core::throw_error(err),
///         }
///     });
///
///     unimplemented!()
/// }
/// ```
pub fn throw_error(error: impl Into<CapturedError> + 'static) {
    Runtime::with(|rt| rt.throw_error(rt.current_scope_id(), error))
}

/// Consume context from the current scope
pub fn try_consume_context<T: 'static + Clone>() -> Option<T> {
    Runtime::with_current_scope(|cx| cx.consume_context::<T>())
}

/// Consume context from the current scope
pub fn consume_context<T: 'static + Clone>() -> T {
    Runtime::with_current_scope(|cx| cx.consume_context::<T>())
        .unwrap_or_else(|| panic!("Could not find context {}", std::any::type_name::<T>()))
}

/// Consume context from the current scope
pub fn consume_context_from_scope<T: 'static + Clone>(scope_id: ScopeId) -> Option<T> {
    Runtime::current()
        .try_get_state(scope_id)
        .and_then(|cx| cx.consume_context::<T>())
}

/// Check if the current scope has a context
pub fn has_context<T: 'static + Clone>() -> Option<T> {
    Runtime::with_current_scope(|cx| cx.has_context::<T>())
}

/// Provide context to the current scope
pub fn provide_context<T: 'static + Clone>(value: T) -> T {
    Runtime::with_current_scope(|cx| cx.provide_context(value))
}

/// Provide a context to the root scope
pub fn provide_root_context<T: 'static + Clone>(value: T) -> T {
    Runtime::with_current_scope(|cx| cx.provide_root_context(value))
}

/// Suspended the current component on a specific task and then return None
pub fn suspend(task: Task) -> Element {
    Err(crate::innerlude::RenderError::Suspended(
        SuspendedFuture::new(task),
    ))
}

/// Start a new future on the same thread as the rest of the VirtualDom.
///
/// **You should generally use `spawn` instead of this method unless you specifically need to run a task during suspense**
///
/// This future will not contribute to suspense resolving but it will run during suspense.
///
/// Because this future runs during suspense, you need to be careful to work with hydration. It is not recommended to do any async IO work in this future, as it can easily cause hydration issues. However, you can use isomorphic tasks to do work that can be consistently replicated on the server and client like logging or responding to state changes.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_core::spawn_isomorphic;
/// // ❌ Do not do requests in isomorphic tasks. It may resolve at a different time on the server and client, causing hydration issues.
/// let mut state = use_signal(|| None);
/// spawn_isomorphic(async move {
///     state.set(Some(reqwest::get("https://api.example.com").await));
/// });
///
/// // ✅ You may wait for a signal to change and then log it
/// let mut state = use_signal(|| 0);
/// spawn_isomorphic(async move {
///     loop {
///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///         println!("State is {state}");
///     }
/// });
/// ```
///
#[doc = include_str!("../docs/common_spawn_errors.md")]
pub fn spawn_isomorphic(fut: impl Future<Output = ()> + 'static) -> Task {
    Runtime::with_current_scope(|cx| cx.spawn_isomorphic(fut))
}

/// Spawns the future and returns the [`Task`]. This task will automatically be canceled when the component is dropped.
///
/// # Example
/// ```rust
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     rsx! {
///         button {
///             onclick: move |_| {
///                 spawn(async move {
///                     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///                     println!("Hello World");
///                 });
///             },
///             "Print hello in one second"
///         }
///     }
/// }
/// ```
///
#[doc = include_str!("../docs/common_spawn_errors.md")]
pub fn spawn(fut: impl Future<Output = ()> + 'static) -> Task {
    Runtime::with_current_scope(|cx| cx.spawn(fut))
}

/// Queue an effect to run after the next render. You generally shouldn't need to interact with this function directly. [use_effect](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html) will call this function for you.
pub fn queue_effect(f: impl FnOnce() + 'static) {
    Runtime::with_current_scope(|cx| cx.queue_effect(f))
}

/// Spawn a future that Dioxus won't clean up when this component is unmounted
///
/// This is good for tasks that need to be run after the component has been dropped.
///
/// **This will run the task in the root scope. Any calls to global methods inside the future (including `context`) will be run in the root scope.**
///
/// # Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_core::spawn_forever;
///
/// // The parent component can create and destroy children dynamically
/// fn App() -> Element {
///     let mut count = use_signal(|| 0);
///
///     rsx! {
///         button {
///             onclick: move |_| count += 1,
///             "Increment"
///         }
///         button {
///             onclick: move |_| count -= 1,
///             "Decrement"
///         }
///
///         for id in 0..10 {
///             Child { id }
///         }
///     }
/// }
///
/// #[component]
/// fn Child(id: i32) -> Element {
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // This will spawn a task in the root scope that will run forever
///                 // It will keep running even if you drop the child component by decreasing the count
///                 spawn_forever(async move {
///                     loop {
///                         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
///                         println!("Running task spawned in child component {id}");
///                     }
///                 });
///             },
///             "Spawn background task"
///         }
///     }
/// }
/// ```
///
#[doc = include_str!("../docs/common_spawn_errors.md")]
pub fn spawn_forever(fut: impl Future<Output = ()> + 'static) -> Task {
    Runtime::with_scope(ScopeId::ROOT, |cx| cx.spawn(fut))
}

/// Informs the scheduler that this task is no longer needed and should be removed.
///
/// This drops the task immediately.
pub fn remove_future(id: Task) {
    Runtime::with(|rt| rt.remove_task(id));
}

/// Store a value between renders. The foundational hook for all other hooks.
///
/// Accepts an `initializer` closure, which is run on the first use of the hook (typically the initial render).
/// `use_hook` will return a clone of the value on every render.
///
/// In order to clean up resources you would need to implement the [`Drop`] trait for an inner value stored in a RC or similar (Signals for instance),
/// as these only drop their inner value once all references have been dropped, which only happens when the component is dropped.
///
/// <div class="warning">
///
/// `use_hook` is not reactive. It just returns the value on every render. If you need state that will track changes, use [`use_signal`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_signal.html) instead.
///
/// ❌ Don't use `use_hook` with `Rc<RefCell<T>>` for state. It will not update the UI and other hooks when the state changes.
/// ```rust
/// use dioxus::prelude::*;
/// use std::rc::Rc;
/// use std::cell::RefCell;
///
/// pub fn Comp() -> Element {
///     let count = use_hook(|| Rc::new(RefCell::new(0)));
///
///     rsx! {
///         button {
///             onclick: move |_| *count.borrow_mut() += 1,
///             "{count.borrow()}"
///         }
///     }
/// }
/// ```
///
/// ✅ Use `use_signal` instead.
/// ```rust
/// use dioxus::prelude::*;
///
/// pub fn Comp() -> Element {
///     let mut count = use_signal(|| 0);
///
///     rsx! {
///         button {
///             onclick: move |_| count += 1,
///             "{count}"
///         }
///     }
/// }
/// ```
///
/// </div>
///
/// # Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// // prints a greeting on the initial render
/// pub fn use_hello_world() {
///     use_hook(|| println!("Hello, world!"));
/// }
/// ```
///
/// # Custom Hook Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// pub struct InnerCustomState(usize);
///
/// impl Drop for InnerCustomState {
///     fn drop(&mut self){
///         println!("Component has been dropped.");
///     }
/// }
///
/// #[derive(Clone, Copy)]
/// pub struct CustomState {
///     inner: Signal<InnerCustomState>
/// }
///
/// pub fn use_custom_state() -> CustomState {
///     use_hook(|| CustomState {
///         inner: Signal::new(InnerCustomState(0))
///     })
/// }
/// ```
#[track_caller]
pub fn use_hook<State: Clone + 'static>(initializer: impl FnOnce() -> State) -> State {
    Runtime::with_current_scope(|cx| cx.use_hook(initializer))
}

/// Get the current render since the inception of this component.
///
/// This can be used as a helpful diagnostic when debugging hooks/renders, etc.
pub fn generation() -> usize {
    Runtime::with_current_scope(|cx| cx.generation())
}

/// Get the parent of the current scope if it exists.
pub fn parent_scope() -> Option<ScopeId> {
    Runtime::with_current_scope(|cx| cx.parent_id())
}

/// Mark the current scope as dirty, causing it to re-render.
pub fn needs_update() {
    Runtime::with_current_scope(|cx| cx.needs_update());
}

/// Mark the current scope as dirty, causing it to re-render.
pub fn needs_update_any(id: ScopeId) {
    Runtime::with_current_scope(|cx| cx.needs_update_any(id));
}

/// Schedule an update for the current component.
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
///
/// Note: The function returned by this method will schedule an update for the current component even if it has already updated between when `schedule_update` was called and when the returned function is called.
/// If the desired behavior is to invalidate the current rendering of the current component (and no-op if already invalidated)
/// [`subscribe`](crate::reactive_context::ReactiveContext::subscribe) to the [`current`](crate::reactive_context::ReactiveContext::current) [`ReactiveContext`](crate::reactive_context::ReactiveContext) instead.
///
/// You should prefer [`schedule_update_any`] if you need to update multiple components.
#[track_caller]
pub fn schedule_update() -> Arc<dyn Fn() + Send + Sync> {
    Runtime::with_current_scope(|cx| cx.schedule_update())
}

/// Schedule an update for any component given its [`ScopeId`].
///
/// A component's [`ScopeId`] can be obtained from the [`current_scope_id`] method.
///
/// Note: Unlike [`needs_update`], the function returned by this method will work outside of the dioxus runtime.
///
/// Note: It does not matter when `schedule_update_any` is called: the returned function will invalidate what ever generation of the specified component is current when returned function is called.
/// If the desired behavior is to schedule invalidation of the current rendering of a component, use [`ReactiveContext`](crate::reactive_context::ReactiveContext) instead.
#[track_caller]
pub fn schedule_update_any() -> Arc<dyn Fn(ScopeId) + Send + Sync> {
    Runtime::with_current_scope(|cx| cx.schedule_update_any())
}

/// Creates a callback that will be run before the component is removed.
/// This can be used to clean up side effects from the component
/// (created with [`use_effect`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html)).
///
/// Note:
/// Effects do not run on the server, but use_drop **DOES**. It runs any time the component is dropped including during SSR rendering on the server. If your clean up logic targets web, the logic has to be gated by a feature, see the below example for details.
///
/// Example:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_core::use_drop;
///
/// fn app() -> Element {
///     let mut state = use_signal(|| true);
///     rsx! {
///         for _ in 0..100 {
///             h1 {
///                 "spacer"
///             }
///         }
///         if state() {
///             child_component {}
///         }
///         button {
///             onclick: move |_| {
///                 state.toggle()
///             },
///             "Unmount element"
///         }
///     }
/// }
///
/// fn child_component() -> Element {
///     let mut original_scroll_position = use_signal(|| 0.0);
///
///     use_effect(move || {
///         let window = web_sys::window().unwrap();
///         let document = window.document().unwrap();
///         let element = document.get_element_by_id("my_element").unwrap();
///         element.scroll_into_view();
///         original_scroll_position.set(window.scroll_y().unwrap());
///     });
///
///     use_drop(move || {
///         // This only make sense to web and hence the `web!` macro
///         web! {
///             /// restore scroll to the top of the page
///             let window = web_sys::window().unwrap();
///             window.scroll_with_x_and_y(original_scroll_position(), 0.0);
///         }
///     });
///
///     rsx! {
///         div {
///             id: "my_element",
///             "hello"
///         }
///     }
/// }
/// ```
#[doc(alias = "use_on_unmount")]
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

    use_hook(|| {
        Rc::new(LifeCycle {
            ondestroy: Some(destroy),
        })
    });
}

/// A hook that allows you to insert a "before render" function.
///
/// This function will always be called before dioxus tries to render your component. This should be used for safely handling
/// early returns
pub fn use_before_render(f: impl FnMut() + 'static) {
    use_hook(|| Runtime::with_current_scope(|cx| cx.push_before_render(f)));
}

/// Push this function to be run after the next render
///
/// This function will always be called before dioxus tries to render your component. This should be used for safely handling
/// early returns
pub fn use_after_render(f: impl FnMut() + 'static) {
    use_hook(|| Runtime::with_current_scope(|cx| cx.push_after_render(f)));
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
