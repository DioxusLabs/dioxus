use dioxus_core::ScopeState;

/// Consume some context in the tree, providing a sharable handle to the value
///
/// Does not regenerate the value if the value is changed at the parent.
#[must_use]
pub fn use_context<T: 'static + Clone>(cx: &ScopeState) -> Option<&T> {
    cx.use_hook(|| cx.consume_context::<T>()).as_ref()
}

/// Provide some context via the tree and return a reference to it
///
/// Once the context has been provided, it is immutable. Mutations should be done via interior mutability.
pub fn use_context_provider<T: 'static + Clone>(cx: &ScopeState, f: impl FnOnce() -> T) -> &T {
    cx.use_hook(|| {
        let val = f();
        cx.provide_context(val.clone());
        val
    })
}
