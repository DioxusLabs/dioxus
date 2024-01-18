use dioxus_core::use_hook;

#[deprecated(
    note = "Use `use_on_destroy` instead, which has the same functionality. \
This is deprecated because of the introduction of `use_on_create` which is better mirrored by `use_on_destroy`. \
The reason why `use_on_create` is not `use_on_mount` is because of potential confusion with `dioxus::events::onmounted`."
)]
pub fn use_on_unmount<D: FnOnce() + 'static>(destroy: D) {
    use_on_destroy(destroy);
}

/// Creates a callback that will be run before the component is removed.
/// This can be used to clean up side effects from the component
/// (created with [`use_effect`](crate::use_effect)).
///
/// Example:
/// ```rust
/// use dioxus::prelude::*;
///
/// fn app() -> Element {
///     let state = use_signal(|| true);
///     rsx! {
///         for _ in 0..100 {
///             h1 {
///                 "spacer"
///             }
///         }
///         if **state {
///             rsx! {
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
/// fn child_component() -> Element {
///     let original_scroll_position = use_signal(|| 0.0);
///     use_effect((), move |_| {
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
///     use_on_destroy({
///         to_owned![original_scroll_position];
///         /// restore scroll to the top of the page
///         move || {
///             let window = web_sys::window().unwrap();
///             window.scroll_with_x_and_y(*original_scroll_position.current(), 0.0);
///         }
///     });
///
///     rsx!{
///         div {
///             id: "my_element",
///             "hello"
///         }
///     }
/// }
/// ```
pub fn use_on_destroy<D: FnOnce() + 'static>(destroy: D) {
    struct LifeCycle<D: FnOnce()> {
        /// Wrap the closure in an option so that we can take it out on drop.
        ondestroy: Option<D>,
    }

    /// On drop, we want to run the closure.
    impl<D: FnOnce()> Drop for LifeCycle<D> {
        fn drop(&mut self) {
            if let Some(f) = self.ondestroy.take() {
                f();
            }
        }
    }

    // We need to impl clone for the lifecycle, but we don't want the drop handler for the closure to be called twice.
    impl<D: FnOnce()> Clone for LifeCycle<D> {
        fn clone(&self) -> Self {
            Self { ondestroy: None }
        }
    }

    use_hook(|| LifeCycle {
        ondestroy: Some(destroy),
    });
}

/// Creates a callback that will be run before the component is dropped
pub fn use_on_drop<D: FnOnce() + 'static>(ondrop: D) {
    use_on_destroy(ondrop);
}

pub fn use_hook_with_cleanup<T: Clone + 'static>(
    hook: impl FnOnce() -> T,
    cleanup: impl FnOnce(T) + 'static,
) -> T {
    let value = use_hook(|| hook());
    let _value = value.clone();
    use_on_destroy(move || cleanup(_value));
    value
}
