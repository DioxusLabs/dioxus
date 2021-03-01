//! Basic example that renders a simple domtree to the browser.

use dioxus_core::prelude::*;
use dioxus_web::*;

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    // Run the app
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |ctx, _| {
    log::info!("Ran component");
    use dioxus::builder::*;
    ctx.render(|b| {
        div(b)
            .child(text("hello"))
            .listeners([on(b, "click", |_| {
                //
                log::info!("button1 clicked!");
            })])
            .finish()
    })
    // ctx.render(html! {
    //     <div onclick={move |_| log::info!("button1 clicked!")}>
    //         "Hello"
    //         // <div class="flex items-center justify-center flex-col">
    //         //     <div class="font-bold text-xl"> "Count is ..." </div>
    //         //     <button onclick={move |_| log::info!("button1 clicked!")}> "increment" </button>
    //         //     <button onclick={move |_| log::info!("button2 clicked!")}> "decrement" </button>
    //         // </div>
    //     </div>
    // })
};
