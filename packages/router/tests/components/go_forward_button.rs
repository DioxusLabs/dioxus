use dioxus::prelude::*;

use crate::{render, test_routes};

#[test]
fn go_forward_button_with_router() {
    render(App);

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                GoForwardButton {
                    "go back"
                }
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic]
fn go_forward_button_without_router_panic_in_debug() {
    render(GoForwardButtonWithoutRouter);
}

#[cfg(not(debug_assertions))]
#[test]
fn go_forward_button_without_router_ignore_in_release() {
    render(GoForwardButtonWithoutRouter);
}

#[allow(non_snake_case)]
fn GoForwardButtonWithoutRouter(cx: Scope) -> Element {
    cx.render(rsx! {
        GoForwardButton {
            "go back"
        }
    })
}
