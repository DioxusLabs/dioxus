//! Reproduce the head element versus `document::eval` ordering gap on desktop.
//!
//! `document::Script` queues the head insertion as an effect, while `document::eval` runs
//! immediately. On web and SSR the script tags are in the first HTML byte, so the probe below sees
//! both flags set. On desktop frame mode the probe can run before the scripts have loaded.
//!
//! Run with `dx serve --example eval_head_ordering` so `asset!` is resolved by the CLI.
//! `cargo run --example eval_head_ordering` also works, but only checks the inline script because
//! `asset!` is left as a placeholder without the CLI.

use dioxus::prelude::*;

const READY: Asset = asset!("/examples/assets/eval_head_ordering.js");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Script { "window.__inline_ready = true;" }
        document::Script { src: READY }
        Probe {}
    }
}

/// Evaluates the globals that only the head scripts define.
#[component]
fn Probe() -> Element {
    use_hook(move || {
        spawn(async move {
            let value = document::eval(
                "return JSON.stringify({ \
                    inline: !!window.__inline_ready, \
                    external: !!window.__ready \
                });",
            )
            .join::<String>()
            .await;
            println!("EVAL_HEAD_ORDERING_RESULT={value:?}");
            dioxus::desktop::window().close();
        });
    });

    rsx! { div { "probe" } }
}
