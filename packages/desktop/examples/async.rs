//! Example: README.md showcase
//!
//! The example from the README.md.

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;

fn main() {
    simple_logger::init().unwrap();
    dioxus_desktop::launch(App, |c| c);
}

static App: FC<()> = |cx, props| {
    let mut count = use_state(cx, || 0);
    log::debug!("count is {:?}", count);

    cx.push_task(|| async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button {
                onclick: move |_| count.set(0),
                "Click me!"
            }
        }
    })
};
