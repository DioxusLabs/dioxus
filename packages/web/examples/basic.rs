//! basic example that renders a simple domtree to the page :)

use dioxus_core::prelude::*;
use dioxus_web::*;

fn main() {
    // Enable logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    // Route panic as console_log
    console_error_panic_hook::set_once();

    // Render the app
    WebsysRenderer::simple_render(html! {
        <div>
            <div class="flex items-center justify-center flex-col">
                <div class="font-bold text-xl"> "Count is {}" </div>
                <button onclick={move |_| log::info!("button1 clicked!")}> "increment" </button>
                <button onclick={move |_| log::info!("button2 clicked!")}> "decrement" </button>
            </div>
        </div>
    });
}
