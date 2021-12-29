//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::use_state;
use dioxus_html as dioxus_elements;
use dioxus_web;
use gloo_timers::future::TimeoutFuture;

fn main() {
    dioxus_web::launch(App);
}

static App: Component = |cx| {
    let mut count = use_state(&cx, || 0);

    cx.push_future(|| async move {
        TimeoutFuture::new(100).await;
        count += 1;
    });

    rsx!(cx, div {
        h3 { "High-Five counter: {count}" }
        button { onclick: move |_| count.set(0), "Reset!" }
    })
};
