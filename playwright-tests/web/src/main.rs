// This test is used by playwright configured in the root of the repo

use dioxus::prelude::*;
use dioxus_web::use_eval;

fn app(cx: Scope) -> Element {
    let mut num = use_state(cx, || 0);
    let eval = use_eval(cx);
    let eval_result = use_state(cx, String::new);

    cx.render(rsx! {
        div {
            "hello axum! {num}"
            button {
                class: "increment-button",
                onclick: move |_| num += 1, "Increment"
            }
        }
        svg {
            circle { cx: 50, cy: 50, r: 40, stroke: "green", fill: "yellow" }
        }
        div {
            class: "raw-attribute-div",
            "raw-attribute": "raw-attribute-value",
        }
        div {
            class: "hidden-attribute-div",
            hidden: true,
        }
        div {
            class: "dangerous-inner-html-div",
            dangerous_inner_html: "<p>hello dangerous inner html</p>",
        }
        input {
            value: "hello input",
        }
        div {
            class: "style-div",
            color: "red",
            "colored text"
        }
        button {
            class: "eval-button",
            onclick: move |_| {
                // Set the window title
                let result = eval(r#"window.document.title = 'Hello from Dioxus Eval!';
                return "returned eval value";"#.to_string());
                if let Ok(serde_json::Value::String(string)) = result.get() {
                    eval_result.set(string);
                }
            },
            "Eval"
        }
        div {
            class: "eval-result",
            "{eval_result}"
        }
    })
}

fn main() {
    dioxus_web::launch(app);
}
