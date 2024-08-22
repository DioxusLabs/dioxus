//! This example shows how to use the `Router` component to create a simple navigation system.
//! The more complex router example uses all of the router features, while this simple example showcases
//! just the `Layout` and `Route` features.
//!
//! Layouts let you wrap chunks of your app with a component. This is useful for things like a footers, heeaders, etc.
//! Routes are enum variants with that match the name of a component in scope. This way you can create a new route
//! in your app simply by adding the variant to the enum and creating a new component with the same name. You can
//! override this of course.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/flat_router.css");

fn main() {
    launch(|| {
        rsx! {
            document::Stylesheet { href: STYLE }
            Router::<Route> {}
        }
    })
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(Footer)] // wrap the entire app in a footer
        #[route("/")]
        Home {},

        #[route("/games")]
        Games {},

        #[route("/play")]
        Play {},

        #[route("/settings")]
        Settings {},
}

#[component]
fn Footer() -> Element {
    rsx! {
        nav {
            Link { to: Route::Home {}, class: "nav-btn", "Home" }
            Link { to: Route::Games {}, class: "nav-btn", "Games" }
            Link { to: Route::Play {}, class: "nav-btn", "Play" }
            Link { to: Route::Settings {}, class: "nav-btn", "Settings" }
        }
        div { id: "content",
            Outlet::<Route> {}
        }
    }
}

#[component]
fn Home() -> Element {
    rsx!(
        h1 { "Home" }
        p { "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." }
    )
}

#[component]
fn Games() -> Element {
    rsx!(
        h1 { "Games" }
        // Dummy text that talks about video games
        p { "Lorem games are sit amet  Sed do eiusmod tempor et dolore magna aliqua." }
    )
}

#[component]
fn Play() -> Element {
    rsx!(
        h1 { "Play" }
        p { "Always play with your full heart adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." }
    )
}

#[component]
fn Settings() -> Element {
    rsx!(
        h1 { "Settings" }
        p { "Settings are consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." }
    )
}
