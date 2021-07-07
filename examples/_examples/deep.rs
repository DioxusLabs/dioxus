use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html_namespace as dioxus_elements;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(CustomA))
}

fn CustomA(cx: Context<()>) -> VNode {
    let (val, set_val) = use_state_classic(cx, || "abcdef".to_string() as String);

    cx.render(rsx! {
        div {
            class: "m-8"
            "CustomA {val}"
            button {
                "Upper"
                onclick: move |_| set_val(val.to_ascii_uppercase())
            }
            button {
                "Lower"
                onclick: move |_| set_val(val.to_ascii_lowercase())
            }
            components::CustomB {
                val: val.clone()
            }
        }
    })
}

mod components {
    use std::rc::Rc;

    use super::*;

    #[derive(Debug, Props, PartialEq)]
    pub struct PropsB {
        val: String,
    }

    pub fn CustomB(cx: Context<PropsB>) -> VNode {
        let (first, last) = cx.val.split_at(3);
        cx.render(rsx! {
            div {
                class: "m-8"
                "CustomB {cx.val}"
                CustomC {
                    val: first.to_string()
                }
                CustomC {
                    val: last.to_string()
                }
            }
        })
    }

    #[derive(Debug, Props, PartialEq)]
    struct PropsC {
        val: String,
    }

    fn CustomC(cx: Context<PropsC>) -> VNode {
        cx.render(rsx! {
            div {
                class: "m-8"
                "CustomC {cx.val}"
            }
        })
    }
}
