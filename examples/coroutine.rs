//! Example: Coroutines!
//! --------------------
//!
//! Coroutines are an awesome way to write concurrent code. Dioxus heavily leverages coroutines to make sense of complex
//! ongoing asynchronous tasks. The async scheduler of Dioxus supports both single-threaded and multi-threaded coroutines,
//! so you can drop in code to run across multiple threads without blocking the main thread.
//!
//! Dioxus cannot simply abstract away the threading model for the web, unfortunately. If you want to use "web threads"
//! you either need to limit support for Chrome, or you need to use a Web Workers and message passing. This is easy enough
//! to do in your own code, and doesn't require 1st-party support from Dioxus itself.
//!
//! UseState and friends work fine with coroutines, but coroutines might be easier to use with the Dirac global state
//! management API. This lets you easily drive global state from a coroutine without having to subscribe to the state.
//!
//! For now, this example shows how to use coroutines used with use_state.
//!
//!
//! ## What is a Coroutine?
//!
//! A coroutine is a function that can be paused and resumed. It can be paused internally through "await" or externally
//! using the `TaskHandle` API. Within a coroutine, you may execute asynchronous code, that modifies values captured when
//! the coroutine was initiated. `use_state` always returns the same setter, so you don't need to worry about

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

use dioxus::prelude::*;

static App: FC<()> = |(cx, props)| {
    let p1 = use_state(cx, || 0);
    let p2 = use_state(cx, || 0);

    let (mut p1_async, mut p2_async) = (p1.for_async(), p2.for_async());
    let (p1_handle, _) = use_task(cx, || async move {
        loop {
            *p1_async.get_mut() += 1;
            async_std::task::sleep(std::time::Duration::from_millis(75)).await;
        }
    });
    let (p2_handle, _) = use_task(cx, || async move {
        loop {
            *p2_async.get_mut() += 1;
            async_std::task::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    cx.render(rsx! {
        div {
            width: "400px", height: "400px", position: "relative", background: "yellow"
            button { "reset", onclick: move |_| {} }
            Horsey { pos: *p1, "horsey 1" }
            Horsey { pos: *p2, "horsey 2" }
        }
    })
};

#[derive(Props)]
struct HorseyProps<'a> {
    pos: i32,
    children: ScopeChildren<'a>,
}

fn Horsey<'a>((cx, props): Scope<'a, HorseyProps<'a>>) -> Element {
    cx.render(rsx! {
        div {
            button { "pause" }
            div {
                {&props.children}
            }
        }
    })
}
