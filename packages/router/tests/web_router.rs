#![cfg(feature = "wasm_test")]
#![allow(non_snake_case)]

use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use gloo::utils::document;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Routable)]
enum Route {
    #[route("/")]
    Home {},
    #[nest("/blog")]
        #[route("/")]
        BlogList {},
        #[route("/:id")]
        BlogPost { id: usize },
}

fn App(cx: Scope) -> Element {
    render!(Router {
        config: RouterConfiguration {
            history: Box::<WebHistory<Route>>::default(),
            ..Default::default()
        }
    })
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Home" }
        }
    })
}

#[inline_props]
fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        div {

        }
    })
}

#[inline_props]
fn BlogPost(cx: Scope, id: usize) -> Element {
    cx.render(rsx! {
        div { }
    })
}

#[wasm_bindgen_test]
fn simple_test() {
    fn main() {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
        dioxus_web::launch(App);
    }

    main();

    let _ = document();
}
