use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: Some(Home),
        dynamic: DynamicRoute::None,
        fixed: vec![(
            String::from("blog"),
            Route {
                name: None,
                component: Blog,
                sub: Some(Segment {
                    index: Some(BlogWelcome),
                    dynamic: DynamicRoute::Variable {
                        name: None,
                        key: "blog_id",
                        component: BlogPost,
                        sub: None,
                    },
                    fixed: vec![],
                }),
            },
        )],
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            header {
                GoBackButton {
                    "go back"
                }
                GoForwardButton {
                    "go forward"
                }
            }
            Outlet { }
        }
    })
}

#[allow(non_snake_case)]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home" }
        Link {
            target: "/blog",
            "go to blog"
        }
    })
}

#[allow(non_snake_case)]
fn Blog(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Blog" }
        Outlet { }
    })
}

#[allow(non_snake_case)]
fn BlogWelcome(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Welcome to the Blog!" }
        ul {
            li{
                Link {
                    target: "/blog/1",
                    "Go to first blog post"
                }
            }
            li{
                Link {
                    target: "/blog/other",
                    "Go to second blog post"
                }
            }
        }
    })
}

#[allow(non_snake_case)]
fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx).expect("called in router");

    let id = route.variables.get("blog_id");
    let title = id
        .map(|id| format!("Blog Post: {id}"))
        .unwrap_or(String::from("Unknown Blog Post"));

    cx.render(rsx! {
        h2 { [title] }
        pre {
           "{id:#?}"
        }
    })
}
