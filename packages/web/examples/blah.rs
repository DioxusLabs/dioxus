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

static App: FC<()> = |cx, props| {
    let mut state = use_state(cx, || 0);
    cx.render(rsx! {
        div {
            style: {
                align_items: "center"
            }
            section { class: "py-12 px-4 text-center"
                div { class: "w-full max-w-2xl mx-auto"
                    span { class: "text-sm font-semibold"
                        "static subtree"
                    }
                }
            }
            section { class: "py-12 px-4 text-center"
                div { class: "w-full max-w-2xl mx-auto"
                    span { class: "text-sm font-semibold"
                        "dynamic subtree {state}"
                    }
                    div {
                        button { onclick: move |_| state+=1, "incr" }
                        br {}
                        button { onclick: move |_| state-=1, "decr" }
                    }
                }
            }
        }
    })
};
