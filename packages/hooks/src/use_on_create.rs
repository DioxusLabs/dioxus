use dioxus_core::ScopeState;
use std::{cell::Cell, future::Future};

/// A hook that runs a future when the component is mounted.
///
/// This is just [`use_effect`](crate::use_effect), but with no dependencies.
/// If you have no dependencies, it's recommended to use this, not just because it's more readable,
/// but also because it's a tiny bit more efficient.
pub fn use_on_create<T, F>(cx: &ScopeState, future: impl FnOnce() -> F)
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let needs_regen = cx.use_hook(|| Cell::new(true));

    if needs_regen.get() {
        needs_regen.set(false);

        let fut = future();

        cx.push_future(async move {
            fut.await;
        });
    }
}
