//! Basic example that renders a simple domtree to the browser.

use dioxus_core::prelude::*;
use dioxus_web::*;

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |ctx, _| {
    ctx.view(html! {
        <div>
            <div class="flex items-center justify-center flex-col">
                <div class="font-bold text-xl"> "Count is {}" </div>
                <button onclick={move |_| log::info!("button1 clicked!")}> "increment" </button>
                <button onclick={move |_| log::info!("button2 clicked!")}> "decrement" </button>
            </div>
        </div>
    })
};
