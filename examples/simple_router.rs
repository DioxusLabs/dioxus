//! A simple example of a router with a few routes and a nav bar.

use dioxus::prelude::*;

fn main() {
    // Launch the router, using our `Route` component as the generic type
    // This will automatically boot the app to "/" unless otherwise specified
    launch(|| rsx! { Router::<Route> {} });
}

/// By default, the Routable derive will use the name of the variant as the route
/// You can also specify a specific component by adding the Component name to the `#[route]` attribute
#[rustfmt::skip]
#[derive(Routable, Clone, PartialEq)]
enum Route {
    // Wrap the app in a Nav layout
    #[layout(Nav)]
        #[route("/")]
        Homepage {},

        #[route("/blog/:id")]
        Blog { id: String },
}

#[component]
fn Homepage() -> Element {
    rsx! {
        h1 { "Welcome home" }
    }
}

#[component]
fn Blog(id: String) -> Element {
    rsx! {
        h1 { "How to make: " }
        p { "{id}" }
    }
}

/// A simple nav bar that links to the homepage and blog pages
///
/// The `Route` enum gives up typesafe routes, allowing us to rename routes and serialize them automatically
#[component]
fn Nav() -> Element {
    rsx! {
        nav {
            li {
                Link { to: Route::Homepage {}, "Go home" }
            }
            li {
                Link {
                    to: Route::Blog {
                        id: "Brownies".to_string(),
                    },
                    "Learn Brownies"
                }
            }
            li {
                Link {
                    to: Route::Blog {
                        id: "Cookies".to_string(),
                    },
                    "Learn Cookies"
                }
            }
        }
        div { Outlet::<Route> {} }
    }
}
