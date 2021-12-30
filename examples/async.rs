/*
This example shows how to use async and loops to implement a coroutine in a component. Coroutines can be controlled via
the `TaskHandle` object.
*/

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[tokio::main]
async fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);
    let direction = use_state(&cx, || 1);

    let (async_count, dir) = (count.for_async(), *direction);

    let task = use_coroutine(&cx, move || async move {
        loop {
            TimeoutFuture::new(250).await;
            *async_count.modify() += dir;
        }
    });

    rsx!(cx, div {
        h1 {"count is {count}"}
        button { onclick: move |_| task.stop(),
            "Stop counting"
        }
        button { onclick: move |_| task.resume(),
            "Start counting"
        }
        button {
            onclick: move |_| {
                *direction.modify() *= -1;
                task.restart();
            },
            "Switch counting direcion"
        }
    })
}
