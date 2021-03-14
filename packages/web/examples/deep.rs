use dioxus_web::{dioxus::prelude::*, WebsysRenderer};

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(CustomA))
}

use components::CustomB;

fn CustomA<'a>(ctx: Context<'a>, props: &'a ()) -> DomTree {
    let (val, set_val) = use_state(&ctx, || "abcdef".to_string());
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
            CustomB {
                val: val
            }
        }
    })
}


mod components {
    use super::*;

    #[derive(Debug, Props, PartialEq)]
    pub struct PropsB<'src> {
        val: &'src str,
    }

    pub fn CustomB<'a>(ctx: Context<'a>, props: &'a PropsB<'a>) -> DomTree {
        let (first, last) = props.val.split_at(3);
        ctx.render(rsx! {
            div {
                class: "m-8"
                "CustomB {props.val}"
                CustomC {
                    val: first
                }
                CustomC {
                    val: last
                }
            }
        })
    }

    #[derive(Debug, Props, PartialEq)]
    struct PropsC<'src> {
        val: &'src str,
    }

    fn CustomC<'a>(ctx: Context<'a>, props: &'a PropsC<'a>) -> DomTree {
        ctx.render(rsx! {
            div {
                class: "m-8"
                "CustomC {props.val}"
            }
        })
    }
}
