use std::marker::PhantomData;

use dioxus::component::Scope;
use dioxus::events::on::MouseEvent;
use dioxus::nodes::{IntoVNode, IntoVNodeList};
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn t() {
    let g = rsx! {
        div {
            div {

            }
        }
    };

    let g = {
        let ___p: Box<dyn FnOnce(NodeFactory) -> VNode> = Box::new(|__cx: NodeFactory| {
            use dioxus_elements::{GlobalAttributes, SvgAttributes};
            __cx.element(dioxus_elements::div, [], [], [], None)
        });
        // let __z = ___p as ;
        // __z
    };
}

fn App((cx, props): Scope<()>) -> Element {
    let a = rsx! {
        div {
            "asd"
        }
    };

    let p = (0..10).map(|f| {
        rsx! {
            div {

            }
        }
    });

    let g = match "text" {
        "a" => rsx!("asd"),
        b => rsx!("asd"),
    };

    let items = ["bob", "bill", "jack"];

    let f = items
        .iter()
        .filter(|f| f.starts_with('b'))
        .map(|f| rsx!("hello {f}"));

    cx.render(rsx! {
        div {
            div {
                {a}
                {p}
                {g}
                {f}
            }
        }
    })
}

// std::boxed::Box<dyn for<'r> std::ops::FnOnce(dioxus_core::NodeFactory<'r>) -> dioxus_core::VNode<'_>>
