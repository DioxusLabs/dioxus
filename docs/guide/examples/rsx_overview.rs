#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

pub fn App(cx: Scope) -> Element {
    cx.render(rsx!(
        Empty {},
        Children {},
        Fragments {},
        Attributes {},
        VariableAttributes {},
        CustomAttributes {},
        Formatting {},
        Expression {},
    ))
}

#[rustfmt::skip]
pub fn Empty(cx: Scope) -> Element {
    // ANCHOR: empty
cx.render(rsx!(div {
    // attributes / listeners
    // children
}))
    // ANCHOR_END: empty
}

#[rustfmt::skip]
pub fn Children(cx: Scope) -> Element {
    // ANCHOR: children
cx.render(rsx!(ol {
    li {"First Item"}
    li {"Second Item"}
    li {"Third Item"}
}))
    // ANCHOR_END: children
}

#[rustfmt::skip]
pub fn Fragments(cx: Scope) -> Element {
    // ANCHOR: fragments
cx.render(rsx!(
    p {"First Item"},
    p {"Second Item"},
    Fragment {
        span { "a group" },
        span { "of three" },
        span { "items" },
    }
))
    // ANCHOR_END: fragments
}

#[rustfmt::skip]
pub fn ManyRoots(cx: Scope) -> Element {
    // ANCHOR: manyroots
cx.render(rsx!(
    p {"First Item"},
    p {"Second Item"},
))
    // ANCHOR_END: manyroots
}

#[rustfmt::skip]
pub fn Attributes(cx: Scope) -> Element {
    // ANCHOR: attributes
cx.render(rsx!(a {
    href: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    class: "primary_button",
    color: "red",
}))
    // ANCHOR_END: attributes
}

#[rustfmt::skip]
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

#[rustfmt::skip]
pub fn CustomAttributes(cx: Scope) -> Element {
    // ANCHOR: custom_attributes
    cx.render(rsx!(b {
        "customAttribute": "value",
    }))
    // ANCHOR_END: custom_attributes
}

#[rustfmt::skip]
pub fn Formatting(cx: Scope) -> Element {
    // ANCHOR: formatting
let coordinates = (42, 0);
let country = "es";
cx.render(rsx!(div {
    class: "country-{country}",
    "position": "{coordinates:?}",
    // arbitrary expressions are allowed,
    // as long as they don't contain `{}`
    div {
        "{country.to_uppercase()}"
    },
    div {
        "{7*6}"
    },
    // {} can be escaped with {{}}
    div {
        "{{}}"
    },
}))
// ANCHOR_END: formatting
}

#[rustfmt::skip]
pub fn Expression(cx: Scope) -> Element {
    // ANCHOR: expression
let text = "Dioxus";
cx.render(rsx!(span {
    text.to_uppercase(),
    // create a list of text from 0 to 9
    (0..10).map(|i| rsx!{ i.to_string() })
}))
    // ANCHOR_END: expression
}

#[rustfmt::skip]
pub fn Loops(cx: Scope) -> Element {
    // ANCHOR: loops
cx.render(rsx!{
    // use a for loop where the body itself is RSX
    div {
        // create a list of text from 0 to 9
        for i in 0..3 {
            // NOTE: the body of the loop is RSX not a rust statement
            div {
                "{i}"
            }
        }
    }
    // iterator equivalent
    div {
        (0..3).map(|i| rsx!{ div { "{i}" } })
    }
})
    // ANCHOR_END: loops
}

#[rustfmt::skip]
pub fn IfStatements(cx: Scope) -> Element {
    // ANCHOR: ifstatements
cx.render(rsx!{
    // use if statements without an else
    if true {
        rsx!(div { "true" })
    }
})
    // ANCHOR_END: ifstatements
}
