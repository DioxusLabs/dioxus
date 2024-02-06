use dioxus_core::{
    prelude::{consume_context, provide_context, try_consume_context},
    use_hook,
};

/// Consume some context in the tree, providing a sharable handle to the value
///
/// Does not regenerate the value if the value is changed at the parent.
#[must_use]
pub fn try_use_context<T: 'static + Clone>() -> Option<T> {
    use_hook(|| try_consume_context::<T>())
}

/// Consume some context in the tree, providing a sharable handle to the value
///
/// Does not regenerate the value if the value is changed at the parent.
#[must_use]
pub fn use_context<T: 'static + Clone>() -> T {
    use_hook(|| consume_context::<T>())
}

/// Provide some context via the tree and return a reference to it
///
/// Once the context has been provided, it is immutable. Mutations should be done via interior mutability.
pub fn use_context_provider<T: 'static + Clone>(f: impl FnOnce() -> T) -> T {
    use_hook(|| {
        let val = f();
        provide_context(val.clone());
        val
    })
}
