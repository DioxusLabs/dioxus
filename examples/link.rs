//! How to use links in Dioxus
//!
//! The `router` crate gives us a `Link` component which is a much more powerful version of the standard HTML link.
//! However, you can use the traditional `<a>` tag if you want to build your own `Link` component.
//!
//! The `Link` component integrates with the Router and is smart enough to detect if the link is internal or external.
//! It also allows taking any `Route` as a target, making your links typesafe

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/links.css");

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! (
        document::Stylesheet { href: STYLE }
        Router::<Route> {}
    )
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(Header)]
        #[route("/")]
        Home {},

        #[route("/default-links")]
        DefaultLinks {},

        #[route("/settings")]
        Settings {},
}

#[component]
fn Header() -> Element {
    rsx! {
        h1 { "Your app here" }
        nav { id: "nav",
            Link { to: Route::Home {}, "home" }
            Link { to: Route::DefaultLinks {}, "default links" }
            Link { to: Route::Settings {}, "settings" }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx!( h1 { "Home" } )
}

#[component]
fn Settings() -> Element {
    rsx!( h1 { "Settings" } )
}

#[component]
fn DefaultLinks() -> Element {
    rsx! {
        // Just some default links
        div { id: "external-links",
            // This link will open in a webbrowser
            a { href: "http://dioxuslabs.com/", "Default link - links outside of your app" }

            // This link will do nothing - we're preventing the default behavior
            // It will just log "Hello Dioxus" to the console
            a {
                href: "http://dioxuslabs.com/",
                onclick: |event| {
                    event.prevent_default();
                    println!("Hello Dioxus")
                },
                "Custom event link - links inside of your app"
            }
        }
    }
}
