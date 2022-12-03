use dioxus_core::ScopeState;

/// Consume some context in the tree
pub fn use_context<T: 'static>(cx: &ScopeState) -> Option<&T> {
    match *cx.use_hook(|| cx.consume_context::<T>().map(|t| t as *const T)) {
        Some(res) => Some(unsafe { &*res }),
        None => None,
    }
}

/// Provide some context via the tree and return a reference to it
///
/// Once the context has been provided, it is immutable. Mutations should be done via interior mutability.
pub fn use_context_provider<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> &T {
    let ptr = *cx.use_hook(|| cx.provide_context(f()) as *const T);
    unsafe { &*ptr }
}
