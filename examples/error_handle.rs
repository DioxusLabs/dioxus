//! This example showcases how to use the ErrorBoundary component to handle errors in your app.
//!
//! The ErrorBoundary component is a special component that can be used to catch panics and other errors that occur.
//! By default, Dioxus will catch panics during rendering, async, and handlers, and bubble them up to the nearest
//! error boundary. If no error boundary is present, it will be caught by the root error boundary and the app will
//! render the error message as just a string.

use dioxus::{dioxus_core::CapturedError, prelude::*};

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: |error: CapturedError| rsx! {
                h1 { "An error occurred" }
                pre { "{error:#?}" }
            },
            DemoC { x: 1 }
        }
    }
}

#[component]
fn DemoC(x: i32) -> Element {
    rsx! {
        h1 { "Error handler demo" }
        button {
            onclick: move |_| {
                // Create an error
                let result: Result<Element, &str> = Err("Error");

                // And then call `throw` on it. The `throw` method is given by the `Throw` trait which is automatically
                // imported via the prelude.
                _ = result.throw();
            },
            "Click to throw an error"
        }
    }
}
