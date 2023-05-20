#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router_core::*;
use dioxus_router_macro::*;

#[inline_props]
fn Route1(cx: Scope, dynamic: String) -> Element {
    render! {
        div{
            "Route1: {dynamic}"
        }
    }
}

#[inline_props]
fn Nested(cx: Scope, nested: String) -> Element {
    render! {
        div{
            "Nested: {nested:?}"
        }
    }
}

#[rustfmt::skip]
#[routable]
#[derive(Clone, Debug, PartialEq)]
enum Route {
    #[nest("/(nested)" nested { nested: String } Nested)]
        #[route("/(dynamic)" Route1)]
        Route1 { dynamic: String },
    #[end_nest]
    #[route("/(dynamic)" Route1)]
    Route2 { dynamic: String },
}
