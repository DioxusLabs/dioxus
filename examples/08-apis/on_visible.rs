//! Port of the https://codepen.io/ryanfinni/pen/VwZeGxN example

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut animated_classes = use_signal(|| ["animated-text", ""]);

    rsx! {
        Stylesheet { href: asset!("/examples/assets/visible.css") }
        div {
            class: "container",

            p {
                "Scroll to the bottom of the page. The text will transition in when it becomes visible in the viewport."
            }

            p {
                "First, let's create a new project for our hacker news app. We can use the CLI to create a new
                project. You can select a platform of your choice or view the getting started guide for more information
                on each option. If you aren't sure what platform to try out, we recommend getting started with web or
                desktop:"
            }

            p {
                "The template contains some boilerplate to help you get started. For this guide, we will be rebuilding some of the code
                from scratch for learning purposes. You can clear the src/main.rs file. We will be adding new code in the next
                sections."
            }

            p {
                "Next, let's setup our dependencies. We need to set up a few dependencies to work with the hacker news API: "
            }

            p {
                "First, let's create a new project for our hacker news app. We can use the CLI to create a new
                project. You can select a platform of your choice or view the getting started guide for more information
                on each option. If you aren't sure what platform to try out, we recommend getting started with web or
                desktop:"
            }

            p {
                "The template contains some boilerplate to help you get started. For this guide, we will be rebuilding some of the code
                from scratch for learning purposes. You can clear the src/main.rs file. We will be adding new code in the next
                sections."
            }

            p {
                "Next, let's setup our dependencies. We need to set up a few dependencies to work with the hacker news API: "
            }

            p {
                "First, let's create a new project for our hacker news app. We can use the CLI to create a new
                project. You can select a platform of your choice or view the getting started guide for more information
                on each option. If you aren't sure what platform to try out, we recommend getting started with web or
                desktop:"
            }

            p {
                "The template contains some boilerplate to help you get started. For this guide, we will be rebuilding some of the code
                from scratch for learning purposes. You can clear the src/main.rs file. We will be adding new code in the next
                sections."
            }

            p {
                "Next, let's setup our dependencies. We need to set up a few dependencies to work with the hacker news API: "
            }

            h2 {
                class: animated_classes().join(" "),
                onvisible: move |evt| {
                    let data = evt.data();
                    if let Ok(is_intersecting) = data.is_intersecting() {
                        animated_classes.write()[1] = if is_intersecting { "visible" } else { "" };
                    }
                },

                "Animated Text"
            }
        }
    }
}
