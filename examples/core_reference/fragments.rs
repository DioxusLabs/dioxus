//! Example: Fragments
//! ------------------
//!
//! Dioxus can return multiple elements without a container through the use of the VNode called a "Fragment". Fragments do not
//! have a mounted root and are inserted inline with their siblings. There are three ways of creating fragments as outlined
//! in the examples below:
//! - By returning multiple elements in Rsx!
//! - By using the `Fragment` component
//! - By using the fragment() method on the node factory

use dioxus::prelude::*;

pub fn Example(cx: Scope) -> Element {
    cx.render(rsx! {
        App1 {}
        App2 {}
        App3 {}
    })
}

// Returning multiple elements with rsx! or html!
pub fn App1(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { }
        h2 { }
        h3 { }
    })
}

// Using the Fragment component
pub fn App2(cx: Scope) -> Element {
    cx.render(rsx! {
        Fragment {
            div {}
            div {}
            "asd"
        }
    })
}

// Using the `fragment` method on the NodeFactory
pub fn App3(cx: Scope) -> Element {
    cx.render(LazyNodes::new(move |fac| {
        fac.fragment_from_iter([
            fac.text(format_args!("A")),
            fac.text(format_args!("B")),
            fac.text(format_args!("A")),
            fac.text(format_args!("B")),
        ])
    }))
}
