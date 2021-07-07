//! Basic example that renders a simple VNode to the browser.

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

static App: FC<()> = |cx| {
    cx.render(rsx! {
        Calcier {
            h2 {"abc 1"}
            h2 {"abc 2"}
            h2 {"abc 3"}
            h2 {"abc 4"}
            h2 {"abc 5"}
        }
    })
};

static Calcier: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            h1 {
                "abc 0"
            }
            {cx.children()}
        }
    })
};
