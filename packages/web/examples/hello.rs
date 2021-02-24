use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    // todo: set this up so the websys render can spawn itself rather than having to wrap it
    // almost like bundling an executor with the wasm version
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

static Example: FC<()> = |ctx, _props| {
    ctx.view(html! {
        <div>
            "Hello world!"
        </div>
    })
};
