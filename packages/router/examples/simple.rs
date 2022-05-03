use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    env_logger::init();
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: TComponent(Home),
        dynamic: DrNone,
        fixed: vec![
            (
                String::from("apple"),
                Route {
                    content: TComponent(Apple),
                    ..Default::default()
                },
            ),
            (
                String::from("potato"),
                Route {
                    name: Some("potato"),
                    content: TComponent(Potato),
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
                    target: NtName("potato", vec![], vec![]),
                    "Read about potatoes…"
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
                target: NtName("root_index", vec![], vec![]),
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
                target: NtName("root_index", vec![], vec![]),
                "Go back to home"
            }
        }
    })
}
