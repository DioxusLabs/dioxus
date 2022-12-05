use dioxus_core::ScopeState;

/// Consume some context in the tree
pub fn use_context<T: 'static>(cx: &ScopeState) -> Option<&T> {
    cx.use_hook(|| cx.consume_context::<T>()).as_deref()
}

/// Provide some context via the tree and return a reference to it
///
/// Once the context has been provided, it is immutable. Mutations should be done via interior mutability.
pub fn use_context_provider<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> &T {
    cx.use_hook(|| cx.provide_context(f()))
}
