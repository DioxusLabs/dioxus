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
fn main() {}

// Returning multiple elements with rsx! or html!
static Example1: FC<()> = |cx| {
    cx.render(rsx! {
        h1 { }
        h2 { }
        h3 { }
        // {}
        // "hello world"
    })
};

// Using the Fragment component
static Example2: FC<()> = |cx| {
    cx.render(rsx! {
        Fragment {
            div {}
            div {}
            "asd"
        }
    })
};

// Using the `fragment` method on the NodeFactory
static Example3: FC<()> = |cx| {
    cx.render(LazyNodes::new(move |fac| {
        fac.fragment_from_iter([
            fac.text(format_args!("A")),
            fac.text(format_args!("B")),
            fac.text(format_args!("A")),
            fac.text(format_args!("B")),
        ])
    }))
};
