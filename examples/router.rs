//! An advanced usage of the router with nested routes and redirects.
//!
//! Dioxus implements an enum-based router, which allows you to define your routes in a type-safe way.
//! However, since we need to bake quite a bit of logic into the enum, we have to add some extra syntax.
//!
//! Note that you don't need to use advanced features like nest, redirect, etc, since these can all be implemented
//! manually, but they are provided as a convenience.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/router.css");

fn main() {
    launch(|| {
        rsx! {
            document::Stylesheet { href: STYLE }
            Router::<Route> {}
        }
    });
}

// Turn off rustfmt since we're doing layouts and routes in the same enum
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    // Wrap Home in a Navbar Layout
    #[layout(NavBar)]
        // The default route is always "/" unless otherwise specified
        #[route("/")]
        Home {},

        // Wrap the next routes in a layout and a nest
        #[nest("/blog")]
        #[layout(Blog)]
            // At "/blog", we want to show a list of blog posts
            #[route("/")]
            BlogList {},

            // At "/blog/:name", we want to show a specific blog post, using the name slug
            #[route("/:name")]
            BlogPost { name: String },

        // We need to end the blog layout and nest
        // Note we don't need either - we could've just done `/blog/` and `/blog/:name` without nesting,
        // but it's a bit cleaner this way
        #[end_layout]
        #[end_nest]

    // And the regular page layout
    #[end_layout]

    // Add some redirects for the `/myblog` route
    #[nest("/myblog")]
        #[redirect("/", || Route::BlogList {})]
        #[redirect("/:name", |name: String| Route::BlogPost { name })]
    #[end_nest]

    // Finally, we need to handle the 404 page
    #[route("/:..route")]
    PageNotFound {
        route: Vec<String>,
    },
}

#[component]
fn NavBar() -> Element {
    rsx! {
        nav { id: "navbar",
            Link { to: Route::Home {}, "Home" }
            Link { to: Route::BlogList {}, "Blog" }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! { h1 { "Welcome to the Dioxus Blog!" } }
}

#[component]
fn Blog() -> Element {
    rsx! {
        h1 { "Blog" }
        Outlet::<Route> {}
    }
}

#[component]
fn BlogList() -> Element {
    rsx! {
        h2 { "Choose a post" }
        div { id: "blog-list",
            Link { to: Route::BlogPost { name: "Blog post 1".into() },
                "Read the first blog post"
            }
            Link { to: Route::BlogPost { name: "Blog post 2".into() },
                "Read the second blog post"
            }
        }
    }
}

// We can use the `name` slug to show a specific blog post
// In theory we could read from the filesystem or a database here
#[component]
fn BlogPost(name: String) -> Element {
    let contents = match name.as_str() {
        "Blog post 1" => "This is the first blog post. It's not very interesting.",
        "Blog post 2" => "This is the second blog post. It's not very interesting either.",
        _ => "This blog post doesn't exist.",
    };

    rsx! {
        h2 { "{name}" }
        p { "{contents}" }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre { color: "red", "log:\nattemped to navigate to: {route:?}" }
    }
}
