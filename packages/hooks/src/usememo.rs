use dioxus_core::ScopeState;

use crate::UseFutureDep;

/// A hook that provides a callback that executes after the hooks have been applied
///
/// Whenever the hooks dependencies change, the callback will be re-evaluated.
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
///
/// ## Examples
///
/// ```rust, ignore
///
/// #[inline_props]
/// fn app(cx: Scope, name: &str) -> Element {
///     use_memo(cx, (name,), |(name,)| {
///         expensive_computation(name);
///     }))
/// }
/// ```
pub fn use_memo<T, D>(cx: &ScopeState, dependencies: D, callback: impl FnOnce(D::Out) -> T) -> &T
where
    T: 'static,
    D: UseFutureDep,
{
    let value = cx.use_hook(|| None);

    let dependancies_vec = cx.use_hook(Vec::new);

    if dependencies.clone().apply(dependancies_vec) || value.is_none() {
        // Create the new value
        *value = Some(callback(dependencies.out()));
    }

    value.as_ref().unwrap()
}
