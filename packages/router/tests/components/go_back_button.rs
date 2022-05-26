use dioxus::prelude::*;

use crate::{render, test_routes};

#[test]
fn go_back_button_with_router() {
    render(App);

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                GoBackButton {
                    "go back"
                }
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic]
fn go_back_button_without_router_panic_in_debug() {
    render(GoBackButtonWithoutRouter);
}

#[cfg(not(debug_assertions))]
#[test]
fn go_back_button_without_router_ignore_in_release() {
    render(GoBackButtonWithoutRouter);
}

#[allow(non_snake_case)]
fn GoBackButtonWithoutRouter(cx: Scope) -> Element {
    cx.render(rsx! {
        GoBackButton {
            "go back"
        }
    })
}
