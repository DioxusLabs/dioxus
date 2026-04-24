//! Accepting children on a component.
//!
//! Any component can take a `children: Element` prop. Anything nested inside the component
//! in `rsx!` is passed in as `children`. This is how you build reusable layout and container
//! components like cards, modals, or page shells.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Card {
            h2 { "A card" }
            p { "Any rsx! can go inside — including other components." }
        }

        Card {
            p { "A second card with different children." }
            button { "Click me" }
        }

        Section { title: "Reusable layout",
            p { "The section component takes both a title prop and children." }
        }
    }
}

#[component]
fn Card(children: Element) -> Element {
    rsx! {
        div {
            border: "1px solid #ccc",
            border_radius: "8px",
            padding: "12px",
            margin: "8px 0",
            {children}
        }
    }
}

#[component]
fn Section(title: String, children: Element) -> Element {
    rsx! {
        section {
            h3 { "{title}" }
            {children}
        }
    }
}
