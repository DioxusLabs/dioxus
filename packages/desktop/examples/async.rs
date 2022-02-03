/* use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let (count, set_count) = use_state(&cx, || 0);

    use_future(&cx, || {
        let set_count = set_count.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                set_count.modify(|f| f + 1)
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            button {
                onclick: move |_| set_count(0),
                "Click me!"
            }
        }
    })
}
 */
