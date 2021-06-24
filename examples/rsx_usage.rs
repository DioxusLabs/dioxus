//! A tour of the rsx! macro
//! ------------------------
//!
//! This example serves as an informal quick reference of all the things that the rsx! macro can do.
//!
//! A full in-depth reference guide is available at: https://www.notion.so/rsx-macro-basics-ef6e367dec124f4784e736d91b0d0b19
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

/// When trying to return "nothing" to Dioxus, you'll need to specify the type parameter or Rust will be sad.
/// This type alias specifices the type for you so you don't need to write "None as Option<()>"
const NONE_ELEMENT: Option<()> = None;

use baller::Baller;
use dioxus_core::prelude::*;

static Example: FC<()> = |cx| {
    let formatting = "formatting!";
    let formatting_tuple = ("a", "b");
    let lazy_fmt = format_args!("lazily formatted text");
    cx.render(rsx! {
        div {
            // Elements
            div {}
            h1 {"Some text"}
            h1 {"Some text with {formatting}"}
            h1 {"Formatting basic expressions {formatting_tuple.0} and {formatting_tuple.1}"}
            h2 {
                "Multiple"
                "Text"
                "Blocks"
                "Use comments as separators in html"
            }
            div {
                h1 {"multiple"}
                h2 {"nested"}
                h3 {"elements"}
            }
            div {
                class: "my special div"
                h1 {"Headers and attributes!"}
            }
            div {
                // pass simple rust expressions in
                class: lazy_fmt,
                id: format_args!("attributes can be passed lazily with std::fmt::Arguments"),
                div {
                    class: {
                        const WORD: &str = "expressions";
                        format_args!("Arguments can be passed in through curly braces for complex {}", WORD)
                    }
                }
            }

            // Expressions can be used in element position too:
            {rsx!(p { "More templating!" })}
            {html!(<p>"Even HTML templating!!"</p>)}

            // Iterators
            {(0..10).map(|i| rsx!(li { "{i}" }))}
            {{
                let data = std::collections::HashMap::<&'static str, &'static str>::new();
                // Iterators *should* have keys when you can provide them.
                // Keys make your app run faster. Make sure your keys are stable, unique, and predictable.
                // Using an "ID" associated with your data is a good idea.
                data.into_iter().map(|(k, v)| rsx!(li { key: "{k}" "{v}" }))
            }}
            

            // Matching
            // Matching will throw a Rust error about "no two closures are the same type"
            // To fix this, call "render" method or use the "in" syntax to produce VNodes.
            // There's nothing we can do about it, sorry :/ (unless you want *really* unhygenic macros)
            {match true {
                true => rsx!(in cx, h1 {"Top text"}),
                false => cx.render(rsx!( h1 {"Bottom text"}))
            }}

            // Conditional rendering
            // Dioxus conditional rendering is based around None/Some. We have no special syntax for conditionals.
            // You can convert a bool condition to rsx! with .then and .or
            {true.then(|| rsx!(div {}))}

            // True conditions need to be rendered (same reasons as matching)
            {if true {
                rsx!(in cx, h1 {"Top text"})
            } else {
                cx.render(rsx!( h1 {"Bottom text"}))
            }}

            // returning "None" is a bit noisy... but rare in practice
            {None as Option<()>}

            // Use the Dioxus type-alias for less noise
            {NONE_ELEMENT}

            // can also just use empty fragments
            Fragment {}

            // Fragments let you insert groups of nodes without a parent.
            // This lets you make components that insert elements as siblings without a container.
            div {"A"}
            Fragment {
                div {"B"}
                div {"C"}
                Fragment {
                    "D"
                    Fragment {
                        "heavily nested fragments is an antipattern"
                        "they cause Dioxus to do unnecessary work"
                        "don't use them carelessly if you can help it"
                    }
                }
            }
            

            // Components
            // Can accept any paths
            // Notice how you still get syntax highlighting and IDE support :)
            Baller {}
            baller::Baller { }
            crate::baller::Baller {}

            // Can take properties
            Taller { a: "asd" }

            // Can take optional properties
            Taller { a: "asd" }

            // Can pass in props directly
            {{
                todo!("this neesd to be implemented");
                let props = TallerProps {a: "hello"};
                rsx!(Taller {a: "a"})
            }}

            // Can take children
            Taller { a: "asd", div {"hello world!"} }
        }
    })
};

mod baller {
    use super::*;
    pub struct BallerProps {}

    /// This component totally balls
    pub fn Baller(cx: Context<()>) -> VNode {
        todo!()
    }
}

#[derive(Debug, PartialEq, Props)]
pub struct TallerProps {
    a: &'static str,
}

/// This component is taller than most :)
pub fn Taller(ctx: Context<TallerProps>) -> VNode {
    let b = true;
    todo!()
}
