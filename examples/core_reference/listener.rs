//! Example: Listeners
//! ------------------
//!
//! This example demonstrates the various ways listeners may be used in Dioxus.
//! Listeners may be at most `FnMut` - but not `FnOnce`.
//! Listeners may borrow the lifetime of cx (children and hooks), but not of local (stack) data.

use dioxus::prelude::*;

pub static Example: Component = |cx| {
    cx.render(rsx! {
        ButtonList {}
        NonUpdatingEvents {}
        DisablePropagation {}
    })
};

/// We can use `set_name` in multiple closures; the closures automatically *copy* the reference to set_name.
static ButtonList: Component = |cx| {
    let name = use_state(&cx, || "...?");

    let names = ["jack", "jill", "john", "jane"]
        .iter()
        .map(move |n| rsx!(button { onclick: move |_| name.set(n), "{n}" }));

    cx.render(rsx!(
        div {
            h1 { "Hello, {name}" }
            {names}
        }
    ))
};

/// This shows how listeners may be without a visible change in the display.
/// Check the console.
static NonUpdatingEvents: Component = |cx| {
    rsx!(cx, div {
        button {
            onclick: move |_| log::info!("Did not cause any updates!")
            "Click me to log!"
        }
    })
};

static DisablePropagation: Component = |cx| {
    rsx!(cx,
        div {
            onclick: move |_| log::info!("event propagated to the div!")
            button {
                onclick: move |evt| {
                    log::info!("Button will allow propagation");
                }
            }
        }
    )
};
