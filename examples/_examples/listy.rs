use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    log::info!("hello world");
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(JonsFavoriteCustomApp));
}

fn JonsFavoriteCustomApp(cx: Context<()>) -> DomTree {
    let items = (0..20).map(|f| {
        rsx! {
            li {"{f}"}
        }
    });

    cx.render(rsx! {
        div {
            "list"
            ul {
                {items}
            }
        }
    })
}
