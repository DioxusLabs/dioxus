use std::rc::Rc;

use dioxus_core as dioxus;
use dioxus_web::{dioxus::prelude::*, WebsysRenderer};

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(CustomA))
}

fn CustomA(ctx: Context<()>) -> VNode {
    let (val, set_val) = use_state(&ctx, || "abcdef".to_string() as String);

    ctx.render(rsx! {
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

    pub fn CustomB(ctx: Context<PropsB>) -> VNode {
        let (first, last) = ctx.val.split_at(3);
        ctx.render(rsx! {
            div {
                class: "m-8"
                "CustomB {ctx.val}"
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

    fn CustomC(ctx: Context<PropsC>) -> VNode {
        ctx.render(rsx! {
            div {
                class: "m-8"
                "CustomC {ctx.val}"
            }
        })
    }
}
