#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;

struct PotatoName;
struct EarthAppleName;

fn main() {
    env_logger::init();
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            .fixed("apple", Route::new(Apple as Component))
            .fixed("potato", Route::new(Potato as Component).name(PotatoName))
            .fixed(
                "earth_apple",
                Route::new((PotatoName, [])).name(EarthAppleName),
            )
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            Outlet { }
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home" }
        p { "Hi! This is a simple example for Dioxus' router:" }
        ul {
            li {
                Link {
                    target: "/apple",
                    "Read about apples…"
                }
            }
            li {
                Link {
                    target: (PotatoName, []),
                    "Read about potatoes…"
                }
            }
            li {
                Link {
                    target: (EarthAppleName, []),
                    "Read about earth apples (literal translation of a german word for potato)…"
                }
            }
        }
    })
}

fn Apple(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Apples" }
        p { "Apples are fruit that grows on trees."}
        p {
            Link {
                target: (RootIndex, []),
                "Go back to home"
            }
        }
    })
}

fn Potato(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Potatoes" }
        p { "Potatoes grow underground. There are many recipes involving potatoes."}
        p {
            Link {
                target: (RootIndex, []),
                "Go back to home"
            }
        }
    })
}
