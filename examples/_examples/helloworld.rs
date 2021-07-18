use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use dioxus_web::prelude::*;

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}

fn App(cx: Context<()>) -> DomTree {
    cx.render(rsx! {
        div { "Hello, world!" }
    })
}
