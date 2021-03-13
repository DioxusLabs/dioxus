use dioxus_web::{WebsysRenderer, dioxus::prelude::*};

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(CustomA))    
}


fn CustomA<'a>(ctx: Context<'a>, props: &'a ()) -> DomTree {
    let (val, set_val) = use_state(&ctx, || "abcdef".to_string());
    ctx.render(rsx!{
        div {
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


#[derive(Debug, Props, PartialEq)]
struct PropsB<'src> {
    val: &'src str
}

fn CustomB<'a>(ctx: Context<'a>, props: &'a PropsB<'a>) -> DomTree {
    let (first, last) = props.val.split_at(3);
    ctx.render(rsx!{
        div {
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
    val: &'src str
}

fn CustomC<'a>(ctx: Context<'a>, props: &'a PropsC<'a>) -> DomTree {
    ctx.render(rsx!{
        div {
            "CustomC {props.val}"
        }
    })
}
