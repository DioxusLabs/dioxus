use dioxus_core::{prelude::consume_context, prelude::provide_root_context, use_hook};

///
pub fn use_root_context<T: 'static + Clone>(new: impl FnOnce() -> T) -> T {
    use_hook(|| {
        consume_context::<T>()
            // If no context is provided, create a new one at the root
            .unwrap_or_else(|| provide_root_context(new()).expect(" A runtime to exist"))
    })
}
