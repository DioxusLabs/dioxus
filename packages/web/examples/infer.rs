use dioxus_core::{events::on::MouseEvent, prelude::*};
use dioxus_web::WebsysRenderer;

fn main() {
    // Setup logging
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

static Example: FC<()> = |ctx, _props| {
    let handler = move |evt: MouseEvent| {
        // Awesome!
        // We get type inference with events
        dbg!(evt.alt_key);
    };

    ctx.render(rsx! {
        button {
            "Hello"
            onclick: {handler}
        }
    })
};
