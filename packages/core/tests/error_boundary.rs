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
        if THREW_ERROR.load(std::sync::atomic::Ordering::SeqCst) {
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
    let out = dioxus_ssr::render(&dom);

    assert_eq!(out, "We should see this");
}
