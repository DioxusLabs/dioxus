use dioxus_core::{prelude::provide_root_context, prelude::try_consume_context, use_hook};

/// Try to get a value from the root of the virtual dom, if it doesn't exist, create a new one with the closure provided.
///
/// This is useful for global context inside of libraries. Instead of having the user provide context in the root of their app, you can use this hook to create a context at the root automatically.
///
/// # Example
/// ```rust
/// # #[derive(Clone)]
/// # struct Logger;
/// use dioxus::prelude::*;
///
/// fn use_logger() -> Logger {
///     // We want one logger per app in the root. Instead of forcing the user to always provide a logger, we can insert a default logger if one doesn't exist.
///     use_root_context(|| Logger)
/// }
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
pub fn use_root_context<T: 'static + Clone>(new: impl FnOnce() -> T) -> T {
    use_hook(|| {
        try_consume_context::<T>()
            // If no context is provided, create a new one at the root
            .unwrap_or_else(|| provide_root_context(new()))
    })
}
