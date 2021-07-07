use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_html_namespace as dioxus_elements;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    log::info!("hello world");
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

static Example: FC<()> = |cx| {
    cx.render(rsx! {
        div {
            span {
                class: "px-2 py-1 flex w-36 mt-4 items-center text-xs rounded-md font-semibold text-yellow-500 bg-yellow-100"
                "DUE DATE : 18 JUN"
            }
        }
    })
};
