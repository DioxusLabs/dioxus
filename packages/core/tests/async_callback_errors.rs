use dioxus::{CapturedError, prelude::*};

#[test]
fn ready_async_callback_error_reaches_error_boundary() {
    #[component]
    fn App() -> Element {
        rsx! {
            ErrorBoundary {
                handle_error: |errors: ErrorContext| rsx! { "caught: {errors.error().unwrap()}" },
                ThrowsReadyError {}
            }
        }
    }

    #[component]
    fn ThrowsReadyError() -> Element {
        use_hook(|| {
            Callback::<()>::new(|()| async {
                Err::<(), _>(CapturedError::from_display("ready error"))
            })
            .call(())
        });
        rsx! { "child content" }
    }

    let mut dom = VirtualDom::new(App);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert_eq!(dioxus_ssr::render(&dom), "caught: ready error");
}
