use dioxus::prelude::*;
use dioxus_router::*;

fn main() {
    // init debug tool for WebAssembly
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus_web::launch(app);
}

fn Works1(cx: Scope) -> Element {
    render!(
        p {
            "this is 1"
        }
        a {
            href: "#section",
            "section"
        }
        Link {
            to: "/2",
            p {
                "go to 2"
            }
        }
        p {
            "{\"AAAA\n\".repeat(999)}"
        }
        h2 {
            id: "section",
            "section"
        }
    )
}

fn Works2(cx: Scope) -> Element {
    render!(
        p {
            "this is 2"
        Link {
            to: "/",
            p {
                "go to 1"
            }
        }
    )
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        Router {
            Route {
                to: "/",
                Works1 {}
            }
            Route {
                to: "/2",
                Works2 {}
            }
        }
    ))
}