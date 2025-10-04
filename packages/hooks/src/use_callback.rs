use dioxus_core::{use_hook, Callback};

/// Create a callback that's always up to date. Whenever this hook is called the inner callback will be replaced with the new callback but the handle will remain.
///
/// There is *currently* no signal tracking on the Callback so anything reading from it will not be updated.
///
/// This API is in flux and might not remain.
#[doc = include_str!("../docs/rules_of_hooks.md")]
pub fn use_callback<T: 'static, O: 'static>(f: impl FnMut(T) -> O + 'static) -> Callback<T, O> {
    let mut callback = Some(f);

    // Create a copyvalue with no contents
    // This copyvalue is generic over F so that it can be sized properly
    let mut inner = use_hook(|| {
        Callback::new(
            callback
                .take()
                .expect("Callback cannot be None on first call"),
        )
    });

    if let Some(callback) = callback.take() {
        // Every time this hook is called replace the inner callback with the new callback
        inner.replace(Box::new(callback));
    }

    inner
}
