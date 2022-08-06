use dioxus::prelude::*;
use dioxus_router::prelude::*;

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
                routes: use_segment(&cx, || {
                    Segment::new().index(Home as Component).fixed("settings", Settings as Component)
                })
                .clone(),

                p { "----"}
                ul {
                    Link { target: "/", li { "Router link to home" } },
                    Link { target: "/settings", li { "Router link to settings" } },
                }
            }
        }
    ))
}

#[allow(non_snake_case)]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        "Home"
    })
}

#[allow(non_snake_case)]
fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        "Settings"
    })
}
