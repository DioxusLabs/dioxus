use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

static Example: FC<()> = |ctx, _props| {
    ctx.render(html! {
        <div>
            "Hello world!"
        </div>
    })
};
