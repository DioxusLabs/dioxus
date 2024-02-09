use dioxus_core::{prelude::provide_root_context, prelude::try_consume_context, use_hook};

///
pub fn use_root_context<T: 'static + Clone>(new: impl FnOnce() -> T) -> T {
    use_hook(|| {
        try_consume_context::<T>()
            // If no context is provided, create a new one at the root
            .unwrap_or_else(|| provide_root_context(new()))
    })
}
