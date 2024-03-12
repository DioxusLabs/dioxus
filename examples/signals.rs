//! A simple example demonstrating how to use signals to modify state from several different places.
//!
//! This simlpe example implements a counter that can be incremented, decremented, and paused. It also demonstrates
//! that background tasks in use_futures can modify the value as well.
//!
//! Most signals implement Into<ReadOnlySignal<T>>, making ReadOnlySignal a good default type when building new
//! library components that don't need to modify their values.

use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut running = use_signal(|| true);
    let mut count = use_signal(|| 0);
    let mut saved_values = use_signal(|| vec![0.to_string()]);

    // use_memo will recompute the value of the signal whenever the captured signals change
    let doubled_count = use_memo(move || count() * 2);

    // use_effect will subscribe to any changes in the signal values it captures
    // effects will always run after first mount and then whenever the signal values change
    use_effect(move || println!("Count changed to {count}"));

    // We can do early returns and conditional rendering which will pause all futures that haven't been polled
    if count() > 30 {
        return rsx! {
            h1 { "Count is too high!" }
            button { onclick: move |_| count.set(0), "Press to reset" }
        };
    }

    // use_future will spawn an infinitely running future that can be started and stopped
    use_future(move || async move {
        loop {
            if running() {
                count += 1;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    // use_resource will spawn a future that resolves to a value
    let _slow_count = use_resource(move || async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        count() * 2
    });

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button { onclick: move |_| running.toggle(), "Toggle counter" }
        button { onclick: move |_| saved_values.push(count.to_string()), "Save this value" }
        button { onclick: move |_| saved_values.clear(), "Clear saved values" }

        // We can do boolean operations on the current signal value
        if count() > 5 {
            h2 { "High five!" }
        }

        // We can cleanly map signals with iterators
        for value in saved_values.iter() {
            h3 { "Saved value: {value}" }
        }

        // We can also use the signal value as a slice
        if let [ref first, .., ref last] = saved_values.read().as_slice() {
            li { "First and last: {first}, {last}" }
        } else {
            "No saved values"
        }

        // You can pass a value directly to any prop that accepts a signal
        Child { count: doubled_count() }
        Child { count: doubled_count }
    }
}

#[component]
fn Child(mut count: ReadOnlySignal<i32>) -> Element {
    println!("rendering child with count {count}");

    rsx! {
        h1 { "{count}" }
    }
}
