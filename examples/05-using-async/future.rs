//! A simple example that shows how to use the use_future hook to run a background task.
//!
//! use_future won't return a value, analogous to use_effect.
//! If you want to return a value from a future, use use_resource instead.

use async_std::task::sleep;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    // use_future is a non-reactive hook that simply runs a future in the background.
    // You can use the UseFuture handle to pause, resume, restart, or cancel the future.
    use_future(move || async move {
        loop {
            sleep(std::time::Duration::from_millis(200)).await;
            count += 1;
        }
    });

    // use_effect is a reactive hook that runs a future when signals captured by its reactive context
    // are modified. This is similar to use_effect in React and is useful for running side effects
    // that depend on the state of your component.
    //
    // Generally, we recommend performing async work in event as a reaction to a user event.
    use_effect(move || {
        spawn(async move {
            sleep(std::time::Duration::from_secs(5)).await;
            count.set(100);
        });
    });

    // You can run futures directly from event handlers as well. Note that if the event handler is
    // fired multiple times, the future will be spawned multiple times.
    rsx! {
        h1 { "Current count: {count}" }
        button {
            onclick: move |_| async move {
                sleep(std::time::Duration::from_millis(200)).await;
                count.set(0);
            },
            "Slowly reset the count"
        }
    }
}
