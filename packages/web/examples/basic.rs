//! Basic example that renders a simple VNode to the browser.

use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
// use wasm_timer;

use std::future::Future;

use std::{pin::Pin, time::Duration};

use dioxus::prelude::*;

use dioxus_web::*;

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    // Run the app
    dioxus_web::launch(App, |c| c)
}

static App: FC<()> = |cx| {
    let mut count = use_state(cx, || 3);

    cx.render(rsx! {
        div {
            button {
                "add"
                onclick: move |_| count += 1
            }
            ul {
                {(0..*count).map(|f| rsx!{
                    li { "a - {f}" }
                    li { "b - {f}" }
                    li { "c - {f}" }
                })}
            }
        }
    })
};
