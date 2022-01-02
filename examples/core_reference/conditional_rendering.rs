//! Example: Conditional Rendering
//! ------------------------------
//!
//! This example shows how to hide or show elements using conditional rendering.
//!
//! Oftentimes you might want to display a different UI given some sort of condition. This is called "conditional rendering".
//! In Dioxus, you can perform conditional rendering with optionals or matching.
//!
//! The rsx! and html! macros accepts anything that is `IntoIter<Item = impl IntoVnode>`. Options and Results both implement
//! IntoIter for impl VNode, so you can use option/result for conditional rendering.

use dioxus::prelude::*;

// Convert a boolean conditional into a hide/show
#[derive(PartialEq, Props)]
pub struct MyProps {
    should_show: bool,
}
pub fn Example0(cx: Scope<MyProps>) -> Element {
    cx.render(rsx! {
        div {
            cx.props.should_show.then(|| rsx!{
                h1 { "showing the title!" }
            })
        }
    })
}

// Convert a boolean conditional into an either/or
// Because rsx! is lazy (produces a closure), we cannot use it in two branch arms. To use it in matching/branching, we
// must render it.
//
// Dioxus will let you render any `LazyNodes` into a `VNode` with `cx.render`. `rsx!` also supports the `in cx` syntax
// which will do essentially the same thing as `cx.render`.
//
// In short:
// `rsx!(cx, ...)` is shorthand for `cx.render(rsx!(...))`
#[derive(PartialEq, Props)]
pub struct MyProps1 {
    should_show: bool,
}
pub fn Example1(cx: Scope<MyProps1>) -> Element {
    cx.render(rsx! {
        div {
            // With matching
            match props.should_show {
                true => cx.render(rsx!(div {"it is true!"})),
                false => rsx!(cx, div {"it is false!"}),
            },

            // or with just regular conditions
            if props.should_show {
                rsx!(cx, div {"it is true!"})
            } else {
                rsx!(cx, div {"it is false!"})
            },

            // or with optional chaining
            props.should_show
                .then(|| rsx!(cx, div {"it is true!"}))
                .unwrap_or_else(|| rsx!(cx, div {"it is false!"}))
        }
    })
}

/// Matching can be expanded

#[derive(PartialEq)]
pub enum Color {
    Green,
    Yellow,
    Red,
}
#[derive(PartialEq, Props)]
pub struct MyProps2 {
    color: Color,
}
pub fn Example2(cx: Scope<MyProps2>) -> Element {
    cx.render(rsx! {
        div {
            match props.color {
                Color::Green => rsx!(cx, div {"it is Green!"}),
                Color::Yellow => rsx!(cx, div {"it is Yellow!"}),
                Color::Red => rsx!(cx, div {"it is Red!"}),
            }
        }
    })
}

pub fn Example(cx: Scope<()>) -> Element {
    let should_show = use_state(&cx, || false);
    let mut color_index = use_state(&cx, || 0);
    let color = match *color_index % 2 {
        2 => Color::Green,
        1 => Color::Yellow,
        _ => Color::Red,
    };

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| should_show.set(!*should_show),
                "click to toggle showing the examples"
            }
            button {
                onclick: move |_| color_index += 1,
                "click to for the enxt color"
            }
            Example0 { should_show: *should_show }
            Example1 { should_show: *should_show }
            Example2 { color: color }
        }
    })
}
