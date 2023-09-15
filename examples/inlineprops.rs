//! Run with `cargo-expand` to see what each one expands to.
//! This file is named `inlineprops.rs`, because there used to be a `#[inline_props]` macro to
//! do this. However, it's now deprecated (and will likely be removed in a future major version),
//! so please use `#[component]` instead!
use dioxus::prelude::*;

#[component]
fn Thing1<T>(cx: Scope, _a: T) -> Element {
    cx.render(rsx! { "" })
}

#[component]
fn Thing2(cx: Scope, _a: u32) -> Element<'a> {
    cx.render(rsx! { "" })
}

#[component]
fn Thing3<'a, T>(cx: Scope<'a>, _a: &'a T) -> Element<'a> {
    cx.render(rsx! { "" })
}

#[component]
fn Thing4<'a>(cx: Scope<'a>, _a: &'a u32) -> Element<'a> {
    cx.render(rsx! { "" })
}

fn main() {
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let state = use_state(cx, || 1);

    cx.render(rsx! {
        div {
            Thing1 { _a: 1 },
            Thing2 { _a: 1 },
            Thing3 { _a: state },
            Thing4 { _a: state },
        }
    })
}
