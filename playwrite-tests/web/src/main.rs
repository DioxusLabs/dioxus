// This test is used by playwrite configured in the root of the repo
use dioxus::prelude::*;

fn app(cx: Scope) -> Element {
    let mut num = use_state(cx, || 0);
    let eval_result = use_state(cx, String::new);

    let eval = dioxus_html::prelude::use_eval(
        cx,
        r#"
            window.document.title = 'Hello from Dioxus Eval!';
            dioxus.send("returned eval value");
            "#,
    );

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
                eval.run().unwrap();
                let receiver = eval.receiver();
                let setter = eval_result.setter();

                wasm_bindgen_futures::spawn_local(async move {
                    let result = receiver.recv().await;
                    if let Ok(serde_json::Value::String(string)) = result {
                        setter(string);
                    }
                });
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
