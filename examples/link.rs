use dioxus::prelude::*;
use dioxus_router::{Link, Route, Router};

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        div {
            p {
                a { href: "http://dioxuslabs.com/", "Default link - links outside of your app" }
            }
            p {
                a {
                    href: "http://dioxuslabs.com/",
                    prevent_default: "onclick",
                    onclick: |_| println!("Hello Dioxus"),
                    "Custom event link - links inside of your app",
                }
            }
        }
        div {
            Router {
                Route { to: "/", h1 { "Home" } },
                Route { to: "/settings", h1 { "settings" } },
                p { "----"}
                ul {
                    Link { to: "/", li { "Router link to home" } },
                    Link { to: "/settings", li { "Router link to settings" } },
                }
            }
        }
    ))
}
