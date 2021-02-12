use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    // todo: set this up so the websys render can spawn itself rather than having to wrap it
    // almost like bundling an executor with the wasm version
    wasm_bindgen_futures::spawn_local(async {
        WebsysRenderer::new(Example)
            .run()
            .await
            .expect("Dioxus Failed! This should *not* happen!")
    });
}

static Example: FC<()> = |ctx, props| {
    ctx.view(html! {
        <div>
            "Hello world!"
        </div>
    })
};
