//! A simple hello world example for Dioxus fullstack with iOS permissions
//!
//! Run with:
//!
//! ```sh
//! dx serve --web
//! dx build --target ios
//! ```
//!
//! This example demonstrates a simple Dioxus fullstack application with a client-side counter
//! and a server function that returns a greeting message. It also includes iOS permissions
//! for camera and location access to demonstrate the permissions system.

use dioxus::prelude::*;
use dioxus_fullstack::get;
use permissions::{permission, Permission};

// Declare iOS permissions for camera and location access
const CAMERA_PERMISSION: Permission = permission!(
    Camera,
    description = "Access camera to take photos and videos for the app"
);

const LOCATION_PERMISSION: Permission = permission!(
    Location(Fine),
    description = "Access location to provide location-based features"
);

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let mut message = use_action(get_greeting);

    rsx! {
        div { style: "padding: 2rem; font-family: Arial, sans-serif;",
            h1 { "Hello, Dioxus Fullstack with iOS Permissions!" }

            // Display permission information
            div { style: "margin: 1rem 0; padding: 1rem; background-color: #f0f0f0; border-radius: 8px;",
                h2 { "📱 iOS Permissions" }
                p { "This app requests the following permissions:" }
                ul {
                    li { "📷 Camera: {CAMERA_PERMISSION.description()}" }
                    li { "📍 Location: {LOCATION_PERMISSION.description()}" }
                }
                p { style: "font-size: 0.9em; color: #666; margin-top: 0.5rem;",
                    "When you build this app for iOS, these permissions will be automatically added to Info.plist"
                }
            }

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
