//! A simple hello world example for Dioxus fullstack
//!
//! Run with:
//!
//! ```sh
//! dx serve --web
//! ```
//!
//! This example demonstrates a simple Dioxus fullstack application with a client-side counter
//! and a server function that returns a greeting message.
//!
//! The `use_action` hook makes it easy to call async work (like server functions) from the client side
//! and handle loading and error states.

use dioxus::prelude::*;
use dioxus_fullstack::get;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let mut message = use_action(get_greeting);

    rsx! {
        div { style: "padding: 2rem; font-family: Arial, sans-serif;",
            h1 { "Hello, Dioxus Fullstack!" }

            // Client-side counter - you can use any client functionality in your app!
            div { style: "margin: 1rem 0;",
                h2 { "Client Counter: {count}" }
                button { onclick: move |_| count += 1, "Increment" }
                button { onclick: move |_| count -= 1, "Decrement" }
            }

            // We can handle the action result and display loading state
            div { style: "margin: 1rem 0;",
                h2 { "Server Greeting" }
                button { onclick: move |_| message.call("World".to_string(), 30), "Get Server Greeting" }
                if message.pending() {
                    p { "Loading..." }
                }
                p { "{message:#?}" }
            }
        }
    }
}

/// A simple server function that returns a greeting
///
/// Our server function takes a name as a path and query parameters as inputs and returns a greeting message.
#[get("/api/greeting/{name}/{age}")]
async fn get_greeting(name: String, age: i32) -> Result<String> {
    Ok(format!(
        "Hello from the server, {}! You are {} years old. 🚀",
        name, age
    ))
}
