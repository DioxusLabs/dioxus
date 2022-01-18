#![allow(non_snake_case)]

use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    dioxus_web::launch(APP);
}

static APP: Component = |cx| {
    cx.render(rsx! {
        Router {
            Route { to: "/", Home {} }
            Route { to: "blog"
                Route { to: "/", BlogList {} }
                Route { to: ":id", BlogPost {} }
            }
        }
    })
};

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Home" }
        }
    })
}

fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        div {

        }
    })
}

fn BlogPost(cx: Scope) -> Element {
    let id = use_route(&cx).segment::<usize>("id")?.ok()?;

    cx.render(rsx! {
        div { "Blog Post {id}" }
    })
}
