use dioxus_core::{consume_context, provide_context, try_consume_context, use_hook};

/// Try to consume some context in the tree, returning `None` if it is not found.
///
/// This is a **hook** — the context value is captured once on the first render and cached
/// for the lifetime of the component. It follows the [rules of hooks](https://dioxuslabs.com/learn/essentials/state/#rules-of-hooks),
/// meaning it must be called unconditionally at the top level of a component.
///
/// If you need to read context from outside a component body (e.g., inside an event handler,
/// async task, or spawned future), use [`try_consume_context`] instead,
/// which can be called anywhere the Dioxus runtime is active.
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[must_use]
pub fn try_use_context<T: 'static + Clone>() -> Option<T> {
    use_hook(|| try_consume_context::<T>())
}

/// Consume some context in the tree, panicking if it is not found.
///
/// This is a **hook** — the context value is captured once on the first render and cached
/// for the lifetime of the component. It follows the [rules of hooks](https://dioxuslabs.com/learn/essentials/state/#rules-of-hooks),
/// meaning it must be called unconditionally at the top level of a component.
///
/// If you need to read context from outside a component body (e.g., inside an event handler,
/// async task, or spawned future), use [`consume_context`] instead,
/// which can be called anywhere the Dioxus runtime is active.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # #[derive(Clone, Copy, PartialEq, Debug)]
/// # enum Theme { Dark, Light }
/// fn Parent() -> Element {
///     use_context_provider(|| Theme::Dark);
///     rsx! { Child {} }
/// }
/// #[component]
/// fn Child() -> Element {
///     //gets context provided by parent element with use_context_provider
///     let user_theme = use_context::<Theme>();
///     rsx! { "user using dark mode: {user_theme == Theme::Dark}" }
/// }
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[must_use]
pub fn use_context<T: 'static + Clone>() -> T {
    use_hook(|| consume_context::<T>())
}

/// Provide some context via the tree and return a reference to it
///
/// Once the context has been provided, it is immutable. Mutations should be done via interior mutability.
/// Context can be read by any child components of the context provider, and is a solution to prop
/// drilling, using a context provider with a Signal inside is a good way to provide global/shared
/// state in your app:
/// ```rust
/// # use dioxus::prelude::*;
///fn app() -> Element {
///    use_context_provider(|| Signal::new(0));
///    rsx! { Child {} }
///}
/// // This component does read from the signal, so when the signal changes it will rerun
///#[component]
///fn Child() -> Element {
///     let mut signal: Signal<i32> = use_context();
///     rsx! {
///         button { onclick: move |_| signal += 1, "increment context" }
///         p {"{signal}"}
///     }
///}
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
pub fn use_context_provider<T: 'static + Clone>(f: impl FnOnce() -> T) -> T {
    use_hook(|| provide_context(f()))
}
