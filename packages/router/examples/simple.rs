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
        index: Some(Home),
        dynamic: DynamicRoute::None,
        fixed: vec![
            (
                String::from("blog"),
                Route {
                    name: None,
                    component: Blog,
                    sub: Some(Segment {
                        index: Some(BlogWelcome),
                        dynamic: DynamicRoute::Variable {
                            name: Some("blog_post"),
                            key: "blog_id",
                            component: BlogPost,
                            sub: None,
                        },
                        fixed: vec![],
                    }),
                },
            ),
            (
                String::from("named_fallback"),
                Route {
                    name: None,
                    component: NamedNavigationFallback,
                    sub: None,
                },
            ),
        ],
    });

    cx.render(rsx! {
        Router {
            named_navigation_fallback_path: String::from("/named_fallback"),
            routes: routes,
            header {
                Link {
                    target: RName("root_index", vec![])
                    "go home"
                }
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
        ul {
            li{
                Link {
                    target: RPath(String::from("/blog")),
                    "go to blog"
                }
            }
            li {
                Link {
                    target: RName("nonexisting name", vec![]),
                    "trigger a named navigation error"
                }
            }
            li {
                Link {
                    target: RExternal(String::from("https://dioxuslabs.com/")),
                    "Go to an external website"
                }
            }
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
            li {
                Link {
                    target: RPath(String::from("/blog/1")),
                    "Go to first blog post"
                }
            }
            li {
                Link {
                    target: RName("blog_post",vec![("blog_id", String::from("2"))]),
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

#[allow(non_snake_case)]
fn NamedNavigationFallback(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Named navigation error" }
        p {
            "Hello user. If you see this, a named navigation operation within this application has "
            "failed. This is a bug. If you encounter this, please write us an email."
        }
    })
}
