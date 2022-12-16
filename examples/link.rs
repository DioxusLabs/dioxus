#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    use_router(cx, &|| RouterConfiguration::default(), &|| {
        Segment::content(comp(Home)).fixed("settings", comp(Settings))
    });

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
            Outlet { }
            p { "----"}
            ul {
                Link { target: "/", li { "Router link to home" } },
                Link { target: "/settings", li { "Router link to settings" } },
            }
        }
    ))
}

fn Home(cx: Scope) -> Element {
    render!(h1 { "Home" })
}

fn Settings(cx: Scope) -> Element {
    render!(h1 { "Settings" })
}
