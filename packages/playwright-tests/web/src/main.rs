// This test is used by playwright configured in the root of the repo

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

fn app() -> Element {
    let mut num = use_signal(|| 0);
    let mut eval_result = use_signal(String::new);

    rsx! {
        div {
            "hello axum! {num}"
            document::Title { "hello axum! {num}" }
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
                // Make sure normal return values work. Regression test for https://github.com/DioxusLabs/dioxus/issues/3655
                let eval = document::eval(r#"return "hello world";"#);

                let result = eval.await.unwrap();
                assert_eq!(result, "hello world");

                // Make sure dioxus.send/dioxus.recv works
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

                // Send 10 numbers
                for i in 0..10 {
                    eval.send(i).unwrap();
                    let value: i32 = eval.recv().await.unwrap();
                    assert_eq!(value, i * 2);
                }

                let result = eval.recv().await;
                if let Ok(serde_json::Value::String(string)) = result {
                    eval_result.set(string);
                }
            },
            "Eval"
        }
        div { class: "eval-result", "{eval_result}" }
        PreventDefault {}
        OnMounted {}
        WebSysClosure {}
        DocumentElements {}
        MergeStyles {}
        SelectMultiple {}
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

#[component]
fn OnMounted() -> Element {
    let mut mounted_triggered_count = use_signal(|| 0);
    rsx! {
        div {
            class: "onmounted-div",
            onmounted: move |_| {
                mounted_triggered_count += 1;
            },
            "onmounted was called {mounted_triggered_count} times"
        }
    }
}

// This component tests attaching an event listener to the document with a web-sys closure
// and effect
#[component]
fn WebSysClosure() -> Element {
    static TRIGGERED: GlobalSignal<bool> = GlobalSignal::new(|| false);
    use_effect(|| {
        let window = web_sys::window().expect("window not available");

        // Assert the component contents have been mounted
        window
            .document()
            .unwrap()
            .get_element_by_id("web-sys-closure-div")
            .expect("Effects should only be run after all contents have bene mounted to the dom");

        // Make sure passing the runtime into the closure works
        let callback = Callback::new(|_| {
            assert!(!dioxus::dioxus_core::Runtime::current().vdom_is_rendering());
            *TRIGGERED.write() = true;
        });
        let closure: Closure<dyn Fn()> = Closure::new({
            move || {
                callback(());
            }
        });

        window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .expect("Failed to add keydown event listener");

        closure.forget();
    });

    rsx! {
        div {
            id: "web-sys-closure-div",
            if TRIGGERED() {
                "the keydown event was triggered"
            }
        }
    }
}

/// This component tests the document::* elements
#[component]
fn DocumentElements() -> Element {
    rsx! {
        document::Meta { id: "meta-head", name: "testing", data: "dioxus-meta-element" }
        document::Link {
            id: "link-head",
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css?family=Roboto+Mono"
        }
        document::Stylesheet { id: "stylesheet-head", href: "https://fonts.googleapis.com/css?family=Roboto:300,300italic,700,700italic" }
        document::Script { id: "script-head", async: true, "console.log('hello world');" }
        document::Style { id: "style-head", "body {{ font-family: 'Roboto'; }}" }

        // Test that links with same href but different rel are NOT deduplicated
        // See https://github.com/DioxusLabs/dioxus/issues/5070
        document::Link {
            id: "dedup-preload",
            rel: "preload",
            href: "dedup-test.css",
            r#as: "style",
        }
        document::Link {
            id: "dedup-stylesheet",
            rel: "stylesheet",
            href: "dedup-test.css",
        }

        // Test that links with same href AND same rel ARE deduplicated
        document::Link {
            id: "dedup-first",
            rel: "stylesheet",
            href: "dedup-same.css",
        }
        document::Link {
            id: "dedup-second",
            rel: "stylesheet",
            href: "dedup-same.css",
        }
    }
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3887
#[component]
fn MergeStyles() -> Element {
    let px = 100;

    rsx! {
        div {
            id: "merge-styles-div",
            style: "width: {px}px; height: {px}px",
            background_color: "red",
        }
    }
}

// Select elements have odd default behavior when you set the multiple attribute after mounting the element
// Regression test for https://github.com/DioxusLabs/dioxus/issues/3185
#[component]
fn SelectMultiple() -> Element {
    rsx! {
        select {
            id: "static-multiple-select",
            // This is static and will be set in the template
            multiple: "true",
            option { label: "Value1", value: "1" }
            option { label: "Value2", value: "2" }
        }

        select {
            id: "dynamic-multiple-select",
            // This is dynamic and will be set after it is mounted
            multiple: true,
            option { label: "Value1", value: "1" }
            option { label: "Value2", value: "2" }
        }
    }
}

fn main() {
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::default()
            .set_max_level(tracing::Level::TRACE)
            .build(),
    );
    dioxus::launch(app);
}
