use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope<()>) -> Element {
    let count = use_state(&cx, || 0);

    // push the futureo on initialization
    cx.use_hook(
        |_| {
            cx.push_future({
                let count = count.for_async();
                async move {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    *count.get_mut() += 1;
                }
            });
        },
        |_| {},
    );

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button {
                onclick: move |_| count.set(0),
                "Click me!"
            }
        }
    })
}
