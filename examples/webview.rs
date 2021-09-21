//! Example: Webview Renderer
//! -------------------------
//!
//! This example shows how to use the dioxus_desktop crate to build a basic desktop application.
//!
//! Under the hood, the dioxus_desktop crate bridges a native Dioxus VirtualDom with a custom prebuit application running
//! in the webview runtime. Custom handlers are provided for the webview instance to consume patches and emit user events
//! into the native VDom instance.
//!
//! Currently, NodeRefs won't work properly, but all other event functionality will.
#![allow(non_upper_case_globals, non_snake_case)]

use dioxus::{events::on::MouseEvent, prelude::*};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    dioxus::desktop::launch(App, |c| c)
}

static App: FC<()> = |cx, props| {
    let state = use_state(cx, || String::from("hello"));
    let clear_text = state == "hello";

    dbg!("rednering parent");
    cx.render(rsx! {
        div {
            h1 {"{state}"}
            CalculatorKey { name: "key-clear", onclick: move |_| state.modify().push_str("hello"), "{clear_text}" }
            CalculatorKey { name: "key-sign", onclick: move |_| { state.modify().pop(); }, "±"}
        }
    })
};

#[derive(Props)]
struct CalculatorKeyProps<'a> {
    name: &'static str,
    onclick: &'a dyn Fn(MouseEvent),
}

fn CalculatorKey<'a>(cx: Context<'a>, props: &'a CalculatorKeyProps) -> DomTree<'a> {
    cx.render(rsx! {
        button {
            class: "calculator-key {props.name}"
            onclick: {props.onclick}
            {cx.children()}
        }
    })
}
