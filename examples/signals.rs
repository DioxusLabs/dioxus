//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use dioxus_signals::{use_init_signal_rt, use_signal};
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    use_init_signal_rt(cx);

    let mut count = use_signal(cx, || 0);

    use_coroutine(cx, |_: UnboundedReceiver<()>| async move {
        loop {
            count += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
