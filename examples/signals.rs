use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut count = dioxus_signals::use_signal(cx, || 0);
    let saved_values = dioxus_signals::use_signal(cx, || vec![0]);

    use_future!(cx, || async move {
        loop {
            count += 1;
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button {
            onclick: move |_| saved_values.push(count.value()),
            "Save this value"
        }

        // We can do boolean operations on the current signal value
        if count.value() > 5 {
            rsx!{ h2 { "High five!" } }
        }

        // We can cleanly map signals with iterators
        for value in saved_values.read().iter() {
            h3 { "Saved value: {value}" }
        }
    })
}
