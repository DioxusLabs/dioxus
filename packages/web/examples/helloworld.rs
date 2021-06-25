use dioxus_core as dioxus;
use dioxus_web::prelude::*;

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}

fn App(ctx: Context<()>) -> VNode {
    ctx.render(rsx! {
        div { "Hello, world!" }
    })
}
