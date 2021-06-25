use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    log::info!("hello world");
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

static Example: FC<()> = |ctx| {
    ctx.render(rsx! {
        div {
            h1 {
                style { color: "red" }
                "Hello style"
            }
            p {"Add a little style"}
        }
    })
};
