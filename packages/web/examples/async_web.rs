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

fn main() {
    // Setup logging
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    dioxus_web::launch(App, |c| c)
}

#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}

const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random/";

static App: FC<()> = |cx| {
    let state = use_state(cx, || 0);

    let dog_node = use_suspense(
        cx,
        || surf::get(ENDPOINT).recv_json::<DogApi>(),
        |cx, res| match res {
            Ok(res) => rsx!(
                cx,
                img {
                    src: "{res.message}"
                }
            ),
            Err(_err) => rsx!(cx, div { "No doggos for you :(" }),
        },
    );

    cx.render(rsx! {
        div { style: { align_items: "center" }
            section { class: "py-12 px-4 text-center"
                div { class: "w-full max-w-2xl mx-auto"
                    span { class: "text-sm font-semibold"
                        "count: {state}"
                    }
                    br {}
                    div {
                        button {
                            onclick: move |_| state.set(state + 1)
                            "incr"
                        }
                        br {}
                        button {
                            onclick: move |_| state.set(state - 1)
                            "decr"
                        }
                    }
                    div {
                        h1{"doggo!"}
                        {dog_node}
                    }
                }
            }
        }
    })
};
