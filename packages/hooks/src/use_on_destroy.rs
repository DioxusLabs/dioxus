#[deprecated(
    note = "Use `use_on_destroy` instead, which has the same functionality. \
This is deprecated because of the introduction of `use_on_create` which is better mirrored by `use_on_destroy`. \
The reason why `use_on_create` is not `use_on_mount` is because of potential confusion with `dioxus::events::onmounted`."
)]
pub fn use_on_unmount<D: FnOnce() + 'static>(cx: &dioxus_core::ScopeState, destroy: D) {
    use_on_destroy(cx, destroy);
}

/// Creates a callback that will be run before the component is removed.
/// This can be used to clean up side effects from the component
/// (created with [`use_effect`](crate::use_effect)).
///
/// Example:
/// ```rust
/// use dioxus::prelude::*;
///
/// fn app(cx: Scope) -> Element {
///     let state = use_state(cx, || true);
///     render! {
///         for _ in 0..100 {
///             h1 {
///                 "spacer"
///             }
///         }
///         if **state {
///             render! {
///                 child_component {}
///             }
///         }
///         button {
///             onclick: move |_| {
///                 state.set(!*state.get());
///             },
///             "Unmount element"
///         }
///     }
/// }
///
/// fn child_component(cx: Scope) -> Element {
///     let original_scroll_position = use_state(cx, || 0.0);
///     use_effect(cx, (), move |_| {
///         to_owned![original_scroll_position];
///         async move {
///             let window = web_sys::window().unwrap();
///             let document = window.document().unwrap();
///             let element = document.get_element_by_id("my_element").unwrap();
///             element.scroll_into_view();
///             original_scroll_position.set(window.scroll_y().unwrap());
///         }
///     });
///
///     use_on_destroy(cx, {
///         to_owned![original_scroll_position];
///         /// restore scroll to the top of the page
///         move || {
///             let window = web_sys::window().unwrap();
///             window.scroll_with_x_and_y(*original_scroll_position.current(), 0.0);
///         }
///     });
///
///     render!{
///         div {
///             id: "my_element",
///             "hello"
///         }
///     }
/// }
/// ```
pub fn use_on_destroy<D: FnOnce() + 'static>(cx: &dioxus_core::ScopeState, destroy: D) {
    cx.use_hook(|| LifeCycle {
        ondestroy: Some(destroy),
    });
}

struct LifeCycle<D: FnOnce()> {
    ondestroy: Option<D>,
}

impl<D: FnOnce()> Drop for LifeCycle<D> {
    fn drop(&mut self) {
        let f = self.ondestroy.take().unwrap();
        f();
    }
}
