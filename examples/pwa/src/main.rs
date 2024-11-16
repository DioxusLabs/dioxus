use dioxus::prelude::*;

fn main() {
    // init debug tool for WebAssembly
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus::launch(app);
}

fn app() -> Element {
    rsx! (
        div { style: "text-align: center;",
            h1 { "🌗 Dioxus 🚀" }
            h3 { "Frontend that scales." }
            p {
                "Dioxus is a Build fullstack web, desktop, and mobile apps with a single codebase.."
            }
        }
    )
}
