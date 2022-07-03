#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

pub fn App(cx: Scope) -> Element {
    cx.render(rsx!(
        Empty {},
        Children {},
        Attributes {},
        VariableAttributes {},
        CustomAttributes {},
        Formatting {},
        Expression {},
    ))
}

pub fn Empty(cx: Scope) -> Element {
    // ANCHOR: empty
    cx.render(rsx!(div {}))
    // ANCHOR_END: empty
}

pub fn Children(cx: Scope) -> Element {
    // ANCHOR: children
    cx.render(rsx!(ol {
        li {"First Item"}
        li {"Second Item"}
        li {"Third Item"}
    }))
    // ANCHOR_END: children
}

pub fn Attributes(cx: Scope) -> Element {
    // ANCHOR: attributes
    cx.render(rsx!(a {
        href: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        class: "primary_button",
        "Log In"
    }))
    // ANCHOR_END: attributes
}

pub fn VariableAttributes(cx: Scope) -> Element {
    // ANCHOR: variable_attributes
    let written_in_rust = true;
    let button_type = "button";
    cx.render(rsx!(button {
        disabled: "{written_in_rust}",
        class: "{button_type}",
        "Rewrite it in rust"
    }))
    // ANCHOR_END: variable_attributes
}

pub fn CustomAttributes(cx: Scope) -> Element {
    println!("???");
    // ANCHOR: custom_attributes
    cx.render(rsx!(b {
        "customAttribute": "value",
        "Rust is Cool"
    }))
    // ANCHOR_END: custom_attributes
}

pub fn Formatting(cx: Scope) -> Element {
    // ANCHOR: formatting
    let coordinates = (42, 0);
    let country = "es";
    cx.render(rsx!(button {
        class: "country-{country}",
        "Coordinates: {coordinates:?}"
    }))
    // ANCHOR_END: formatting
}

pub fn Expression(cx: Scope) -> Element {
    // ANCHOR: expression
    let text = "Dioxus";
    cx.render(rsx!(span {
        [text.to_uppercase()]
    }))
    // ANCHOR_END: expression
}
