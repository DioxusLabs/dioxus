//! Simple single-page-app setup.
//!
//!  Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::{logger::tracing, prelude::*};
use wasm_bindgen::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut t = use_signal(|| 0);

    rsx! {
        h1 { "Set your favorite color" }
        button { onclick: move |_| t += 1, "Click me: {t}" }
        button {
            onclick: move |_| {
                let items = get_select_data_list("hello".to_string());
                tracing::debug!("items: {:?}", items);
            },
            "Get select data"
        }
        div {
            EvalIt { color: "red" }
            EvalIt { color: "orange" }
            EvalIt { color: "yellow" }
            EvalIt { color: "green" }
            EvalIt { color: "blue" }
            EvalIt { color: "indigo" }
            EvalIt { color: "violet" }
            EvalIt { color: "red" }
            EvalIt { color: "orange" }
            EvalIt { color: "yellow" }
            EvalIt { color: "green" }
            EvalIt { color: "blue" }
            EvalIt { color: "indigo" }
            EvalIt { color: "violet" }
            EvalIt { color: "black" }
        }
    }
}

#[component]
fn EvalIt(color: String) -> Element {
    rsx! {
        div {
            button {
                onclick: move |_| {
                    _ = dioxus::document::eval(&format!("window.document.body.style.backgroundColor = '{color}';"));
                },
                "eval -> {color}"
            }
        }
    }
}

#[wasm_bindgen(inline_js = r#"
export function get_select_data_list(select) {
    let values = [select];

    return values;
}
"#)]
extern "C" {
    pub fn get_select_data_list(item: String) -> Vec<String>;
}
