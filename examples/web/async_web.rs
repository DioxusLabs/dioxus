//! Basic example that renders a simple VNode to the browser.
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    dioxus_web::launch(APP)
}

#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}

static APP: Component = |(cx, _props)| {
    let state = use_state(&cx, || 0);

    const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random/";
    let doggo = use_suspense(
        cx,
        || async { reqwest::get(ENDPOINT).await.unwrap().json::<DogApi>().await },
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

    rsx!(cx, div { align_items: "center"
        section { class: "py-12 px-4 text-center"
            div { class: "w-full max-w-2xl mx-auto"
                span { class: "text-sm font-semibold"
                    "count: {state}"
                }
                div {
                    button { onclick: move |_| state.set(state + 1), "incr" }
                    button { onclick: move |_| state.set(state - 1), "decr" }
                }
                div {
                    {doggo}
                }
            }
        }
    })
};
