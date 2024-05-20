use dioxus_core::{prelude::provide_root_context, prelude::try_consume_context, use_hook};

/// Try to get a value from the root of the virtual dom, if it doesn't exist, create a new one with the closure provided.
pub fn use_root_context<T: 'static + Clone>(new: impl FnOnce() -> T) -> T {
    use_hook(|| {
        try_consume_context::<T>()
            // If no context is provided, create a new one at the root
            .unwrap_or_else(|| provide_root_context(new()))
    })
}
