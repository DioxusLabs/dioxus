#![cfg(feature = "wasm_test")]
#![allow(non_snake_case)]

use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use gloo::utils::document;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn simple_test() {
    fn main() {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
        dioxus_web::launch(App);
    }

    fn App(cx: Scope) -> Element {
        use_router(
            cx,
            &|| RouterConfiguration {
                on_update: Some(Arc::new(|_| None)),
                ..Default::default()
            },
            &|| {
                Segment::content(comp(Home)).fixed(
                    "blog",
                    Route::empty().nested(
                        Segment::content(comp(BlogList)).catch_all((comp(BlogPost, PostId {}))),
                    ),
                )
            },
        );

        render!(Outlet {})
    }

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

    struct PostId;
    fn BlogPost(cx: Scope) -> Element {
        let _id = use_route(cx)?.parameter::<PostId>().unwrap();

        cx.render(rsx! {
            div { }
        })
    }

    main();

    let _ = document();
}
