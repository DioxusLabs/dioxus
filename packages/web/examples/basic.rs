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
    let mut content = use_state(cx, || String::from("h1"));
    let mut text_content = use_state(cx, || String::from("Hello, world!"));

    log::debug!("running scope...");

    cx.render(rsx! {
        div {
            h1 { "content val is {content}" }

            input {
                r#type: "text",
                value: "{text_content}"
                oninput: move |e| text_content.set(e.value())
            }

            br {}
            {(0..10).map(|f| {
                rsx!(
                    button {
                        onclick: move |_| count += 1,
                        "Click to add."
                        "Current count: {count}"
                    }
                    br {}
                )
            })}

            select {
                name: "cars"
                id: "cars"
                value: "{content}"
                oninput: move |ev| {
                    content.set(ev.value());
                    match ev.value().as_str() {
                        "h1" => count.set(0),
                        "h2" => count.set(5),
                        "h3" => count.set(10),
                        _ => {}
                    }
                },

                option { value: "h1", "h1" }
                option { value: "h2", "h2" }
                option { value: "h3", "h3" }
            }

            {render_list(cx, *count)}

            {render_bullets(cx)}

            Child {}
        }
    })
};

fn render_bullets(cx: Context) -> DomTree {
    rsx!(cx, div {
        "bite me"
    })
}

fn render_list(cx: Context, count: usize) -> DomTree {
    let items = (0..count).map(|f| {
        rsx! {
            li { "a - {f}" }
            li { "b - {f}" }
            li { "c - {f}" }
        }
    });

    rsx!(cx, ul { {items} })
}

static Child: FC<()> = |cx, props| {
    // render
    rsx!(cx, div {"hello child"})
};
