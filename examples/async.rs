/*
This example shows how to use async and loops to implement a coroutine in a component. Coroutines can be controlled via
the `TaskHandle` object.
*/

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[tokio::main]
async fn main() {
    dioxus::desktop::launch(App);
}

pub static App: Component = |cx| {
    let count = use_state(&cx, || 0);
    let mut direction = use_state(&cx, || 1);

    let (async_count, dir) = (count.for_async(), *direction);

    let task = use_coroutine(&cx, move || async move {
        loop {
            TimeoutFuture::new(250).await;
            *async_count.get_mut() += dir;
        }
    });

    rsx!(cx, div {
        h1 {"count is {count}"}
        button {
            "Stop counting"
            onclick: move |_| task.stop()
        }
        button {
            "Start counting"
            onclick: move |_| task.resume()
        }
        button {
            "Switch counting direcion"
            onclick: move |_| {
                direction *= -1;
                task.restart();
            }
        }
    })
};
