use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::prelude::*;

fn main() {
    env_logger::init();
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        dynamic: DrNone,
        fixed: vec![
            (
                String::from("apple"),
                Route {
                    content: RcComponent(Apple),
                    ..Default::default()
                },
            ),
            (
                String::from("potato"),
                Route {
                    name: Some("potato"),
                    content: RcComponent(Potato),
                    ..Default::default()
                },
            ),
            (
                String::from("earth_apple"),
                Route {
                    name: Some("earth apple"),
                    content: RcRedirect(NtName("potato", vec![], QNone)),
                    ..Default::default()
                },
            ),
        ],
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            Outlet { }
        }
    })
}

#[allow(non_snake_case)]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home" }
        p { "Hi! This is a simple example for Dioxus' router:" }
        ul {
            li {
                Link {
                    target: NtPath(String::from("/apple")),
                    "Read about apples…"
                }
            }
            li {
                Link {
                    target: NtName("potato", vec![], QNone),
                    "Read about potatoes…"
                }
            }
            li {
                Link {
                    target: NtName("earth apple", vec![], QNone),
                    "Read about earth apples (literal translation of a german word for potato)…"
                }
            }
        }
    })
}

#[allow(non_snake_case)]
fn Apple(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Apples" }
        p { "Apples are fruit that grows on trees."}
        p {
            Link {
                target: NtName("root_index", vec![], QNone),
                "Go back to home"
            }
        }
    })
}

#[allow(non_snake_case)]
fn Potato(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Potatoes" }
        p { "Potatoes grow underground. There are many recipes involving potatoes."}
        p {
            Link {
                target: NtName("root_index", vec![], QNone),
                "Go back to home"
            }
        }
    })
}
