//! Basic example that renders a simple VNode to the browser.

use std::rc::Rc;

use dioxus_core::prelude::*;
use dioxus_web::*;
fn main() {
    // Setup logging
    // wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    // Run the app
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}

static App: FC<()> = |cx| {
    let (contents, set_contents) = use_state(&cx, || "asd");

    cx.render(rsx! {
        div  {
            class: "flex items-center justify-center flex-col"
            div {
                class: "flex items-center justify-center"
                div {
                    class: "flex flex-col bg-white rounded p-4 w-full max-w-xs"
                    div { class: "font-bold text-xl", "Example cloud app" }
                    div { class: "text-sm text-gray-500", "This is running in the cloud!!" }
                    div {
                        class: "flex flex-row items-center justify-center mt-6"
                        div { class: "font-medium text-6xl", "100%" }
                    }
                    div {
                        class: "flex flex-row justify-between mt-6"
                        a {
                            href: "https://www.dioxuslabs.com"
                            class: "underline"
                            "Made with dioxus"
                        }
                    }
                }
            }
        }
    })
};
