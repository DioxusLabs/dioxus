#![cfg(target_arch = "wasm32")]
#![allow(non_snake_case)]

use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;
use gloo_utils::document;
use serde::{Deserialize, Serialize};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn simple_test() {
    fn main() {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
        dioxus_web::launch(APP);
    }

    static APP: Component = |cx| {
        cx.render(rsx! {
            Router {
                onchange: move |route: RouterService| log::trace!("route changed to {:?}", route.current_location()),
                active_class: "is-active",
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
        let id = use_route(&cx).parse_segment::<usize>("id")?;

        cx.render(rsx! {
            div {

            }
        })
    }

    main();

    let element = gloo_utils::document();
}
