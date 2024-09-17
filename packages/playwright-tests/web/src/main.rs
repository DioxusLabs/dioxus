// This test is used by playwright configured in the root of the repo

use dioxus::prelude::*;

fn app() -> Element {
    let mut num = use_signal(|| 0);
    let mut eval_result = use_signal(String::new);

    rsx! {
        div {
            document::Title { "hello axum! {num}" }
            "hello axum! {num}"
            button { class: "increment-button", onclick: move |_| num += 1, "Increment" }
        }
        svg { circle { cx: 50, cy: 50, r: 40, stroke: "green", fill: "yellow" } }
        div { class: "raw-attribute-div", "raw-attribute": "raw-attribute-value" }
        div { class: "hidden-attribute-div", hidden: true }
        div {
            class: "dangerous-inner-html-div",
            dangerous_inner_html: "<p>hello dangerous inner html</p>"
        }
        input { value: "hello input" }
        div { class: "style-div", color: "red", "colored text" }
        button {
            class: "eval-button",
            onclick: move |_| async move {
                let mut eval = document::eval(
                    r#"
                        window.document.title = 'Hello from Dioxus Eval!';
                        // Receive and multiply 10 numbers
                        for (let i = 0; i < 10; i++) {
                            let value = await dioxus.recv();
                            dioxus.send(value*2);
                        }
                        dioxus.send("returned eval value");
                    "#,
                );

                todo!("no more dioxus evaluator thing anymore - just a plain-old evail")
                // // Send 10 numbers
                // for i in 0..10 {
                //     eval.send(serde_json::Value::from(i)).unwrap();
                //     let value = eval.recv().await.unwrap();
                //     assert_eq!(value, serde_json::Value::from(i * 2));
                // }

                // let result = eval.recv().await;
                // if let Ok(serde_json::Value::String(string)) = result {
                //     eval_result.set(string);
                // }
            },
            "Eval"
        }
        div { class: "eval-result", "{eval_result}" }
        PreventDefault {}
    }
}

#[component]
fn PreventDefault() -> Element {
    let mut text = use_signal(|| "View source".to_string());
    rsx! {
        a {
            class: "prevent-default",
            href: "https://github.com/DioxusLabs/dioxus/tree/main/packages/playwright-tests/web",
            onclick: move |evt| {
                evt.prevent_default();
                text.set("Psych!".to_string());
            },
            "{text}"
        }
    }
}

fn main() {
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::default()
            .set_max_level(tracing::Level::TRACE)
            .build(),
    );
    launch(app);
}
