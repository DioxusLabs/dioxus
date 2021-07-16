//! Example: Tasks
//! --------------
//!
//! Built around the same system that powers suspense, Dioxus also allows users to write arbitrary tasks that can modify
//! state asynchronously. `use_task` takes a future and returns a task handle and an option that holds the tasks's return
//! value. When the task completes, the component will re-render with the result freshly available.
//!
//! Tasks don't necessarily need to complete, however. It would be completely reasonable to wire up a websocket receiver
//! in a task and have it work infinitely while the app is running. If the socket throws an error, the task could complete
//! and the UI could present a helpful error message.
//!
//! Tasks also return the `TaskHandle` type which lets other component logic start, stop, and restart the task at any time.
//! Tasks are very much like an async-flavoroued coroutine, making them incredibly powerful.
//!
//! Tasks must be valid for the 'static lifetime, so any state management neeeds to be cloned into the closure. `use_state`
//! has a method called `for_async` which generates an AsyncUseState wrapper. This has a very similar API to the regualr
//! `use_state` but is `static.
//!
//! Remeber `use_task` is a hook! Don't call it out of order or in loops. You might aaccidentally swap the task handles
//! and break things in subtle ways.
//!
//! Whenever a component is scheduled for deletion, the task is dropped. Make sure that whatever primitives used in the
//! task have a valid drop implementation and won't leave resources tied up.

use dioxus::prelude::*;

pub static Example: FC<()> = |cx| {
    let count = use_state(cx, || 0);
    let mut direction = use_state(cx, || 1);

    // Tasks are 'static, so we need to copy relevant items in
    let (async_count, dir) = (count.for_async(), *direction);
    let (task, result) = cx.use_task(move || async move {
        // Count infinitely!
        loop {
            gloo_timers::future::TimeoutFuture::new(250).await;
            *async_count.get_mut() += dir;
        }
    });

    cx.render(rsx! {
        div {
            h1 {"count is {count}"}
            button {
                "Stop counting"
                onclick: move |_| task.stop()
            }
            button {
                "Start counting"
                onclick: move |_| task.start()
            }
            button {
                "Switch counting direcion"
                onclick: move |_| {
                    direction *= -1;
                    task.restart();
                }
            }
        }
    })
};
