//! Basic example that renders a simple VNode to the browser.

// all these imports are done automatically with the `dioxus` crate and `prelude`
// need to do them manually for this example
use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;

use dioxus::prelude::*;
use dioxus_web::*;
use std::future::Future;
use std::{pin::Pin, time::Duration};

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    // Run the app
    dioxus_web::launch(APP, |c| c)
}

static APP: FC<()> = |cx, props| {
    let mut count = use_state(cx, || 3);

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| count += 1,
                "Click to add."
                "Current count: {count}"
            }
            select {
                name:"cars"
                id:"cars"
                oninput: move |ev| {
                    match ev.value().as_str() {
                        "h1" => count.set(0),
                        "h2" => count.set(5),
                        "h3" => count.set(10),
                        s => {
                            log::debug!("real value is {}", s);
                        }
                    }
                },
                option { value: "h1", "h1" }
                option { value: "h2", "h2" }
                option { value: "h3", "h3" }
            }

            ul {
                {(0..*count).map(|f| rsx!{
                    li { "a - {f}" }
                    li { "b - {f}" }
                    li { "c - {f}" }
                })}
            }
            Child {}
        }
    })
};

static Child: FC<()> = |cx, props| {
    cx.render(rsx! {
        div {
            div {
                "hello child"
            }
        }
    })
};
