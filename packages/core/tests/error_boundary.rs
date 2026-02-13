#![allow(non_snake_case)]

use dioxus::{prelude::*, CapturedError};

#[test]
fn catches_panic() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Title" }

            NoneChild {}
            ThrowChild {}
        }
    }
}

fn NoneChild() -> Element {
    VNode::empty()
}

fn ThrowChild() -> Element {
    Err(std::io::Error::new(std::io::ErrorKind::AddrInUse, "asd"))?;

    let _g: i32 = "123123".parse()?;

    rsx! { div {} }
}

#[test]
fn clear_error_boundary() {
    static THREW_ERROR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    #[component]
    fn App() -> Element {
        rsx! {
            AutoClearError {}
        }
    }

    #[component]
    pub fn ThrowsError() -> Element {
        if !THREW_ERROR.load(std::sync::atomic::Ordering::SeqCst) {
            THREW_ERROR.store(true, std::sync::atomic::Ordering::SeqCst);
            Err(CapturedError::from_display("This is an error").into())
        } else {
            rsx! {
                "We should see this"
            }
        }
    }

    #[component]
    pub fn AutoClearError() -> Element {
        rsx! {
            ErrorBoundary {
                handle_error: |error: ErrorContext| {
                    error.clear_errors();

                    rsx! { "We cleared it" }
                },

                ThrowsError {}
            }
        }
    }

    let mut dom = VirtualDom::new(App);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    // The DOM will now contain no text - the error was thrown by ThrowsError and caught by the
    // ErrorBoundary. This calls `needs_update()`, but the update hasn't been processed yet.

    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    // The ErrorBoundary has now called its `handle_error` handler. The DOM contains the string
    // "We cleared it", and `needs_update()` has been called again (by `error.clear_errors()`)

    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    // `ThrowsError` is re-rendered, but this time does not throw an error, so at the end the DOM
    // contains "We should see this"

    let out = dioxus_ssr::render(&dom);
    assert_eq!(out, "We should see this");

    // There should be no errors left in the error boundary
    dom.in_runtime(|| {
        ScopeId::APP.in_runtime(|| assert!(consume_context::<ErrorContext>().errors().is_empty()))
    })
}
