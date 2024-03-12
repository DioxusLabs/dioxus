use dioxus_core::{
    prelude::{consume_context, provide_context, try_consume_context},
    use_hook,
};

/// Consume some context in the tree, providing a sharable handle to the value
///
/// Does not regenerate the value if the value is changed at the parent.
#[must_use]
pub fn try_use_context<T: 'static + Clone>() -> Option<T> {
    use_hook(|| try_consume_context::<T>())
}

/// Consume some context in the tree, providing a sharable handle to the value
///
/// Does not regenerate the value if the value is changed at the parent.
/// ```rust
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
///fn app() -> Element {
///    use_context_provider(|| Signal::new(0));
///    rsx! { Child {} }
///}
/// // This component does read from the signal, so when the signal changes it will rerun
///#[component]
///fn Child() -> Element {
///     let signal: Signal<i32> = use_context();
///     rsx! {
///         button { onclick: move |_| signal += 1, "increment context" }
///         p {"{signal}"}
///     }
///}
/// ```
pub fn use_context_provider<T: 'static + Clone>(f: impl FnOnce() -> T) -> T {
    use_hook(|| {
        let val = f();
        provide_context(val.clone());
        val
    })
}
