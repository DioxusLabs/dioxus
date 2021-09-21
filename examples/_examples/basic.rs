//! Basic example that renders a simple VNode to the browser.

use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_web::*;

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    // Run the app
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |cx, props| {
    let (state, set_state) = use_state_classic(cx, || 0);
    cx.render(rsx! {
        div {
            section { class: "py-12 px-4 text-center"
                div { class: "w-full max-w-2xl mx-auto"
                    span { class: "text-sm font-semibold"
                        "count: {state}"
                    }
                    div {
                        C1 {
                            onclick: move |_| set_state(state + 1)
                            "incr"
                        }
                        C1 {
                            onclick: move |_| set_state(state - 1)
                            "decr"
                        }
                    }
                }
            }
        }
    })
};

#[derive(Props)]
struct IncrementerProps<'a> {
    onclick: &'a dyn Fn(MouseEvent),
}

fn C1<'a, 'b>(cx: Context<'a, IncrementerProps<'b>>) -> DomTree<'a> {
    cx.render(rsx! {
        button {
            class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
            onclick: {cx.onclick}
            "becr"
            {cx.children()}
        }
    })
}
