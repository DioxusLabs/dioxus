use dioxus_core::ScopeState;

use crate::UseFutureDep;

/// A hook that provides a callback that executes if the dependencies change.
/// This is useful to avoid running computation-expensive calculations even when the data doesn't change.
///
/// - dependencies: a tuple of references to values that are `PartialEq` + `Clone`
///
/// ## Examples
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
///
/// #[component]
/// fn Calculator(cx: Scope, number: usize) -> Element {
///     let bigger_number = use_memo(cx, (number,), |(number,)| {
///         // This will only be calculated when `number` has changed.
///         number * 100
///     });
///     render!(
///         p { "{bigger_number}" }
///     )
/// }
///
/// #[component]
/// fn App(cx: Scope) -> Element {
///     render!(Calculator { number: 0 })
/// }
/// ```
#[must_use = "Consider using `use_effect` to run rerun a callback when dependencies change"]
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
