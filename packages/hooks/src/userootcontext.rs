use dioxus_core::ScopeState;

///
pub fn use_root_context<T: 'static + Clone>(cx: &ScopeState, new: impl FnOnce() -> T) -> &T {
    cx.use_hook(|| {
        cx.consume_context::<T>()
            .unwrap_or_else(|| cx.provide_root_context(new()))
    })
}
