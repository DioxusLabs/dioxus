/// Creates a new SetCompare which efficiently tracks when a value changes to check if it is equal to a set of values.
///
/// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App() -> Element {
///     let mut count = use_signal(cx, || 0);
///     let compare = use_set_compare(cx, move || count.value());
///
///     render! {
///         for i in 0..10 {
///             // Child will only re-render when i == count
///             Child { active: compare.equal(i) }
///         }
///         button {
///             // This will only rerender the child with the old and new value of i == count
///             // Because we are using a set compare, this will be O(1) instead of the O(n) performance of a selector
///             onclick: move |_| count += 1,
///             "Increment"
///         }
///     }
/// }
///
/// #[component]
/// fn Child(active: ReadOnlySignal<bool>) -> Element {
///     if *active() {
///         render! { "Active" }
///     } else {
///         render! { "Inactive" }
///     }
/// }
/// ```
#[must_use]
pub fn use_set_compare<R: Eq + Hash>(f: impl FnMut() -> R + 'static) -> SetCompare<R> {
    use_hook(move || SetCompare::new(f))
}
