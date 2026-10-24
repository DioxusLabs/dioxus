//! Defining and using components.
//!
//! A component is just a function annotated with `#[component]` that returns an `Element`.
//! Its arguments are its props — Dioxus generates a builder so you can construct the
//! component inside `rsx!` with a struct-like syntax.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        h1 { "Greetings" }
        Greeting { name: "Alice" }
        Greeting { name: "Bob", excited: true }
        Greeting { name: "Charlie", excited: false }

        h2 { "User cards" }
        UserCard { name: "Ada Lovelace", role: "Mathematician" }
        UserCard { name: "Grace Hopper", role: "Computer Scientist" }
    }
}

// Components must start with an uppercase letter.
// Props are taken as named arguments — `name` is required, and `excited` has a default.
#[component]
fn Greeting(name: String, #[props(default)] excited: bool) -> Element {
    let punctuation = if excited { "!" } else { "." };
    rsx! {
        p { "Hello, {name}{punctuation}" }
    }
}

// Components can take any Clone + PartialEq type as a prop.
#[component]
fn UserCard(name: String, role: String) -> Element {
    rsx! {
        div { border: "1px solid #ccc", padding: "8px", margin: "4px",
            strong { "{name}" }
            " — "
            em { "{role}" }
        }
    }
}
