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
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |cx| {
    // let mut count = use_state(cx, || 0);
    let state = use_state(cx, || 0);
    let set_val = state.setter();

    let g = cx.use_task(|| async move {
        let mut tick: i32 = 0;
        log::debug!("yeet!");
        loop {
            gloo_timers::future::TimeoutFuture::new(250).await;
            log::debug!("ticking forward... {}", tick);
            tick += 1;
            if tick > 10 {
                break;
            }
        }
        set_val(10);
        String::from("Huzza!")
    });

    log::debug!("Value from component was {:#?}", g);

    cx.render(rsx! {
        div {
            section { class: "py-12 px-4 text-center"
                div { class: "w-full max-w-2xl mx-auto"
                    span { class: "text-sm font-semibold"
                        "count: {state}"
                    }
                    div {
                        button {
                            onclick: move |_| state.set(state + 1)
                            "incr"
                        }
                        br {}
                        br {}
                        button {
                            onclick: move |_| state.set(state - 1)
                            "decr"
                        }
                    }
                }
            }
        }
    })
};
