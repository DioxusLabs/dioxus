#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

pub fn App(cx: Scope) -> Element {
    let is_logged_in = use_state(cx, || false);

    cx.render(rsx!(LogIn {
        is_logged_in: **is_logged_in,
        on_log_in: |_| is_logged_in.set(true),
        on_log_out: |_| is_logged_in.set(false),
    }))
}

#[inline_props]
fn LogIn<'a>(
    cx: Scope<'a>,
    is_logged_in: bool,
    on_log_in: EventHandler<'a>,
    on_log_out: EventHandler<'a>,
) -> Element<'a> {
    // ANCHOR: if_else
    if *is_logged_in {
        cx.render(rsx! {
            div {
                "Welcome!",
                button {
                    onclick: move |_| on_log_out.call(()),
                    "Log Out",
                }
            }
        })
    } else {
        cx.render(rsx! {
            button {
                onclick: move |_| on_log_in.call(()),
                "Log In",
            }
        })
    }
    // ANCHOR_END: if_else
}

#[inline_props]
fn LogInWarning(cx: Scope, is_logged_in: bool) -> Element {
    // ANCHOR: conditional_none
    if *is_logged_in {
        return cx.render(rsx!(()));
    }

    cx.render(rsx! {
        a {
            "You must be logged in to comment"
        }
    })
    // ANCHOR_END: conditional_none
}
