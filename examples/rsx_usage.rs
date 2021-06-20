//! A tour of the rsx! macro
//! ------------------------
//!
//! This example serves as an informal quick reference of all the things that the rsx! macro can do.
//!
//! A full in-depth reference guide is available at: https://www.notion.so/rsx-macro-basics-ef6e367dec124f4784e736d91b0d0b19
//!
//! ## Topics
//!
//!
//!
//! ### Elements
//! - Create any element from its tag
//! - Accept compile-safe attributes for each tag
//! - Display documentation for elements
//! - Arguments instead of String
//! - Text
//! - Inline Styles
//!
//! ## General Concepts
//! - Iterators
//! - Keys
//! - Match statements
//! - Conditional Rendering
//!
//! ### Events
//! - Handle events with the "onXYZ" syntax
//! - Closures can capture their environment with the 'a lifetime
//!
//!
//! ### Components
//! - Components can be made by specifying the name
//! - Components can be referenced by path
//! - Components may have optional parameters
//! - Components may have their properties specified by spread syntax
//! - Components may accept child nodes
//! - Components that accept "onXYZ" get those closures bump allocated
//!
//! ### Fragments
//! - Allow fragments using the built-in `Fragment` component
//! - Accept a list of vnodes as children for a Fragment component
//! - Allow keyed fragments in iterators
//! - Allow top-level fragments
//!
fn main() {
    dioxus::webview::launch(Example);
}

use baller::Baller;
use dioxus_core::prelude::*;

static Example: FC<()> = |ctx| {
    ctx.render(rsx! {
        div {
            // Elements




            // ==============
            //   Components
            // ==============
            // Can accept any paths
            crate::baller::Baller {}
            baller::Baller { }

            // Can take properties
            Taller { a: "asd" }

            // Can take optional properties
            Taller { a: "asd" }

            // Can pass in props directly
            Taller { a: "asd" /* ..{props}*/  }

            // Can take children
            Taller { a: "asd", div {} }
        }
    })
};

mod baller {
    use super::*;
    pub struct BallerProps {}

    pub fn Baller(ctx: Context<()>) -> VNode {
        todo!()
    }
}

#[derive(Debug, PartialEq, Props)]
pub struct TallerProps {
    a: &'static str,
}

pub fn Taller(ctx: Context<TallerProps>) -> VNode {
    let b = true;
    todo!()
}
