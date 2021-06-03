#![allow(non_upper_case_globals)]

use dioxus_ssr::{
    prelude::*,
    prelude::{builder::IntoVNode, dioxus::events::on::MouseEvent},
    TextRenderer,
};
mod utils;

fn main() {
    let renderer = TextRenderer::new(App);
}

static App: FC<()> = |ctx| {
    rsx! { in ctx,
        div {
            Home {}
            Docs {}
            Tutorial {}
            Blog {}
            Community {}
        }
    }
};

const HeroContent: [(&'static str, &'static str); 3] = [
    ("Declarative", 
    "React makes it painless to create interactive UIs. Design simple views for each state in your application, and React will efficiently update and render just the right components when your data changes.\nDeclarative views make your code more predictable and easier to debug."),

    ("Component-Based", "Build encapsulated components that manage their own state, then compose them to make complex UIs.\nSince component logic is written in JavaScript instead of templates, you can easily pass rich data through your app and keep state out of the DOM."),

    ("Learn Once, Write Anywhere", "We don’t make assumptions about the rest of your technology stack, so you can develop new features in React without rewriting existing code.\nReact can also render on the server using Node and power mobile apps using React Native."),
];

const SnippetHighlights: &'static str = include_str!("./snippets.md");

static Home: FC<()> = |ctx| {
    let hero = HeroContent.iter().map(|(title, body)| {
        rsx! {
            div {
                h3 { "{title}" }
                div { {body.split("\n").map(|paragraph| rsx!( p{"{paragraph}"} ))} }
            }
        }
    });
    let snippets: Vec<VNode> = utils::markdown_to_snippet(ctx, SnippetHighlights);

    rsx! { in ctx,
        div {
            header {
                // Hero
                section {
                    div { {hero} }
                }
                hr {}
                // Highlighted Snippets
                section {
                    {snippets}
                }
            }
            div {}
            section {}
        }
    }
};

static Docs: FC<()> = |ctx| {
    rsx! { in ctx,
        div {

        }
    }
};

static Tutorial: FC<()> = |ctx| {
    rsx! { in ctx,
        div {

        }
    }
};

static Blog: FC<()> = |ctx| {
    rsx! { in ctx,
        div {

        }
    }
};

static Community: FC<()> = |ctx| {
    rsx! { in ctx,
        div {

        }
    }
};
