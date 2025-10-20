//! Regression test for <https://github.com/DioxusLabs/dioxus/issues/4646>

use dioxus::prelude::*;

fn main() {
    // we split these two to ensure `dioxus::serve` works properly.
    #[cfg(feature = "server")]
    dioxus::serve(|| async move { Ok(dioxus::server::router(app)) });

    #[cfg(not(feature = "server"))]
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Comp {}
        Comp {}
        Button {}
    }
}

#[component]
fn Button() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        button {
            id: "counter",
            onclick: move |_| {
                count += 1;
            },
            "Count: {count}"
        }
    }
}

#[component]
fn Comp(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    rsx! {
        div {
            width: 100,
            div {
                ..attributes,
            }
        }
    }
}
