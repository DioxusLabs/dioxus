//! Run with `cargo-expand` to see what each one expands to
#![allow(non_snake_case)]

use dioxus::prelude::*;

#[inline_props]
fn Thing1<T>(cx: Scope, _a: T) -> Element {
    cx.render(rsx! { "" })
}

#[inline_props]
fn Thing2(cx: Scope, _a: u32) -> Element<'a> {
    cx.render(rsx! { "" })
}

#[inline_props]
fn Thing3<'a, T>(cx: Scope<'a>, _a: &'a T) -> Element<'a> {
    cx.render(rsx! { "" })
}

#[inline_props]
fn Thing4<'a>(cx: Scope<'a>, _a: &'a u32) -> Element<'a> {
    cx.render(rsx! { "" })
}

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
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
