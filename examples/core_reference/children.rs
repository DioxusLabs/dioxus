//! Example: Children
//! -----------------
//!
//! Dioxus supports passing children in from the parent. These children are allocated in the parent and just passed
//! into the child. Components that pass in children may not be safely memoized, though in practice it's rare for a
//! change in a parent to not result in a different set of children.
//!
//! In Dioxus, children can *only be a list*. Unlike React, you cannot pass in functions or arbitrary data. This is
//! partially a limitation of having static types, but is rather intentional to encourage the use of attributes where
//! arbitrary child data might normally be used. Check out the `function driven children` example for how to adopt your
//! React pattern to Dioxus' semantics.
//!
//! Dioxus will let you use the `children` method more than once - and it's semantically *okay* - but you'll likely
//! ruin your page if you try to clone elements in this way. Under the hood, Dioxus shares a "mounted ID" for each node,
//! and mounting the same VNode in two places will overwrite the first mounted ID. This will likely lead to dead elements.
//!
//! In the future, this might become a runtime error, so consider it an error today.

use dioxus::prelude::*;

pub static Example: Component = |cx| {
    cx.render(rsx! {
        div {
            Banner {
                p { "Some Content1" }
            }
            Banner {
                p { "Some Content2" }
            }
        }
    })
};

pub static Banner: Component = |cx| {
    cx.render(rsx! {
        div {
            h1 { "This is a great banner!" }
            div { class: "content"
                {cx.children()}
            }
            footer { "Wow, what a great footer" }
        }
    })
};
