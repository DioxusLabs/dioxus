use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let running = dioxus_signals::use_signal(cx, || true);
    let mut count = dioxus_signals::use_signal(cx, || 0);
    let saved_values = dioxus_signals::use_signal(cx, || vec![0.to_string()]);

    // Signals can be used in async functions without an explicit clone since they're 'static and Copy
    // Signals are backed by a runtime that is designed to deeply integrate with Dioxus apps
    use_future!(cx, || async move {
        loop {
            if running.value() {
                count += 1;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button { onclick: move |_| running.toggle(), "Toggle counter" }
        button { onclick: move |_| saved_values.push(count.value().to_string()), "Save this value" }
        button { onclick: move |_| saved_values.write().clear(), "Clear saved values" }

        // We can do boolean operations on the current signal value
        if count.value() > 5 {
            rsx!{ h2 { "High five!" } }
        }

        // We can cleanly map signals with iterators
        for value in saved_values.read().iter() {
            h3 { "Saved value: {value}" }
        }

        // We can also use the signal value as a slice
        if let [ref first, .., ref last] = saved_values.read().as_slice() {
            rsx! { li { "First and last: {first}, {last}" } }
        } else {
            rsx! { "No saved values" }
        }
    })
}
