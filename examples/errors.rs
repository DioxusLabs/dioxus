//! This example showcases how to use the ErrorBoundary component to handle errors in your app.
//!
//! The ErrorBoundary component is a special component that can be used to catch panics and other errors that occur.
//! By default, Dioxus will catch panics during rendering, async, and handlers, and bubble them up to the nearest
//! error boundary. If no error boundary is present, it will be caught by the root error boundary and the app will
//! render the error message as just a string.
//!
//! NOTE: In wasm, panics can currently not be caught by the error boundary. This is a limitation of WASM in rust.
#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    launch(|| rsx! { Router::<Route> {} });
}

/// You can use an ErrorBoundary to catch errors in children and display a warning
fn Simple() -> Element {
    rsx! {
        GoBackButton { "Home" }
        ErrorBoundary {
            handle_error: |error: ErrorContext| rsx! {
                h1 { "An error occurred" }
                pre { "{error:#?}" }
            },
            ParseNumber {}
        }
    }
}

#[component]
fn ParseNumber() -> Element {
    rsx! {
        h1 { "Error handler demo" }
        button {
            onclick: move |_| {
                // You can return a result from an event handler which lets you easily quit rendering early if something fails
                let data: i32 = "0.5".parse()?;

                println!("parsed {data}");

                Ok(())
            },
            "Click to throw an error"
        }
    }
}

// You can provide additional context for the Error boundary to visualize
fn Show() -> Element {
    rsx! {
        GoBackButton { "Home" }
        div {
            ErrorBoundary {
                handle_error: |errors: ErrorContext| {
                    rsx! {
                        for error in errors.errors() {
                            if let Some(error) = error.show() {
                                {error}
                            } else {
                                pre {
                                    color: "red",
                                    "{error}"
                                }
                            }
                        }
                    }
                },
                ParseNumberWithShow {}
            }
        }
    }
}

#[component]
fn ParseNumberWithShow() -> Element {
    rsx! {
        h1 { "Error handler demo" }
        button {
            onclick: move |_| {
                let request_data = "0.5";
                let data: i32 = request_data.parse()
                    // You can attach rsx to results that can be displayed in the Error Boundary
                    .show(|_| rsx! {
                        div {
                            background_color: "red",
                            border: "black",
                            border_width: "2px",
                            border_radius: "5px",
                            p { "Failed to parse data" }
                            Link {
                                to: Route::Home {},
                                "Go back to the homepage"
                            }
                        }
                    })?;

                println!("parsed {data}");

                Ok(())
            },
            "Click to throw an error"
        }
    }
}

// On desktop, dioxus will catch panics in components and insert an error automatically
fn Panic() -> Element {
    rsx! {
        GoBackButton { "Home" }
        ErrorBoundary {
            handle_error: |errors: ErrorContext| rsx! {
                h1 { "Another error occurred" }
                pre { "{errors:#?}" }
            },
            ComponentPanic {}
        }
    }
}

#[component]
fn ComponentPanic() -> Element {
    panic!("This component panics")
}

#[derive(Routable, Clone, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/simple")]
    Simple {},
    #[route("/panic")]
    Panic {},
    #[route("/show")]
    Show {},
}

fn Home() -> Element {
    rsx! {
        ul {
            li {
                Link {
                    to: Route::Simple {},
                    "Simple errors"
                }
            }
            li {
                Link {
                    to: Route::Panic {},
                    "Capture panics"
                }
            }
            li {
                Link {
                    to: Route::Show {},
                    "Show errors"
                }
            }
        }
    }
}
