//! Basic example that renders a simple VNode to the browser.

use dioxus_core::prelude::*;
use dioxus_web::*;

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    // Run the app
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |ctx| {
    ctx.render(rsx! {
        h2 { "abc 1" }
        div {
            "hello world!"
        }
        Fragment {
            h2 { "abc 2"}
        }
    })
};
