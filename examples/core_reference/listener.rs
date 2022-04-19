//! Example: Listeners
//! ------------------
//!
//! This example demonstrates the various ways listeners may be used in Dioxus.
//! Listeners may be at most `FnMut` - but not `FnOnce`.
//! Listeners may borrow the lifetime of cx (children and hooks), but not of local (stack) data.

use dioxus::prelude::*;

pub fn Example(cx: Scope) -> Element {
    cx.render(rsx! {
        ButtonList {}
        NonUpdatingEvents {}
        DisablePropagation {}
    })
}

/// We can use `set_name` in multiple closures; the closures automatically *copy* the reference to set_name.
pub fn ButtonList(cx: Scope) -> Element {
    let name = use_state(&cx, || "...?");

    cx.render(rsx!(
        div {
            h1 { "Hello, {name}" }

            ["jack", "jill", "john", "jane"]
                .iter()
                .map(move |n| rsx!(button { onclick: move |_| name.set(n), "{n}" }))
        }
    ))
}

/// This shows how listeners may be without a visible change in the display.
/// Check the console.
pub fn NonUpdatingEvents(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            button {
                onclick: move |_| log::trace!("Did not cause any updates!"),
                "Click me to log!"
            }
        }
    })
}

pub fn DisablePropagation(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            onclick: move |_| log::trace!("event propagated to the div!")
            button {
                onclick: move |evt| log::trace!("Button will allow propagation"),
            }
        }
    })
}
