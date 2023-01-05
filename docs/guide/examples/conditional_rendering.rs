#![allow(unused)]
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
#[rustfmt::skip]
fn LogIn<'a>(
    cx: Scope<'a>,
    is_logged_in: bool,
    on_log_in: EventHandler<'a>,
    on_log_out: EventHandler<'a>,
) -> Element<'a> {
    // ANCHOR: if_else
if *is_logged_in {
    cx.render(rsx! {
        "Welcome!"
        button {
            onclick: move |_| on_log_out.call(()),
            "Log Out",
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
#[rustfmt::skip]
fn LogInImproved<'a>(
    cx: Scope<'a>,
    is_logged_in: bool,
    on_log_in: EventHandler<'a>,
    on_log_out: EventHandler<'a>,
) -> Element<'a> {
    // ANCHOR: if_else_improved
cx.render(rsx! {
    // We only render the welcome message if we are logged in
    // You can use if statements in the middle of a render block to conditionally render elements
    if *is_logged_in {
        // Notice the body of this if statment is rsx code, not an expression
        "Welcome!"
    }
    button {
        // depending on the value of `is_logged_in`, we will call a different event handler
        onclick: move |_| if *is_logged_in {
            on_log_in.call(())
        }
        else{
            on_log_out.call(())
        },
        if *is_logged_in {
            // if we are logged in, the button should say "Log Out"
            "Log Out"
        } else {
            // if we are not logged in, the button should say "Log In"
            "Log In"
        }
    }
})
    // ANCHOR_END: if_else_improved
}

#[inline_props]
#[rustfmt::skip]
fn LogInWarning(cx: Scope, is_logged_in: bool) -> Element {
    // ANCHOR: conditional_none
if *is_logged_in {
    return None;
}

cx.render(rsx! {
    a {
        "You must be logged in to comment"
    }
})
    // ANCHOR_END: conditional_none
}
