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
        index: RcComponent(Home),
        dynamic: DrNone,
        fixed: vec![
            (
                String::from("blog"),
                Route {
                    name: None,
                    content: RcComponent(Blog),
                    sub: Some(Segment {
                        index: RcComponent(BlogWelcome),
                        dynamic: DrParameter {
                            name: Some("blog_post"),
                            key: "blog_id",
                            content: RcComponent(BlogPost),
                            sub: None,
                        },
                        fixed: vec![],
                    }),
                },
            ),
            (
                String::from("raspberry"),
                Route {
                    name: Some("raspberry"),
                    content: RcMulti(RaspberryPage, vec![("other", StrawberryPage)]),
                    sub: None,
                },
            ),
            (
                String::from("the_best_berry"),
                Route {
                    name: Some("best_berry"),
                    content: RcRedirect(ItName("raspberry", vec![], QNone)),
                    sub: None,
                },
            ),
            (
                String::from("named_fallback"),
                Route {
                    name: None,
                    content: RcComponent(NamedNavigationFallback),
                    sub: None,
                },
            ),
        ],
    });

    cx.render(rsx! {
        style {
            r#"
                .active {{
                    color: red;
                }}
                .other {{
                    color: white;
                    background-color: blue;
                    float: right;
                }}
            "#
        }
        Router {
            active_class: "active",
            named_navigation_fallback_path: String::from("/named_fallback"),
            routes: routes,
            header {
                Link {
                    target: NtName("root_index", vec![], QNone)
                    "go home"
                }
                GoBackButton {
                    "go back"
                }
                GoForwardButton {
                    "go forward"
                }
                PathDisplay {}
                Outlet { name: "other" }
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
                    target: NtPath(String::from("/blog")),
                    "go to blog"
                }
            }
            li {
                Link {
                    target: NtName("nonexisting name", vec![], QNone),
                    "trigger a named navigation error"
                }
            }
            li {
                Link {
                    target: NtExternal(String::from("https://dioxuslabs.com/")),
                    "Go to an external website"
                }
            }
            li {
                Link {
                    target: NtName("raspberry", vec![], QNone),
                    "Go to the page about raspberries"
                }
            }
            li {
                Link {
                    target: NtName("best_berry", vec![], QNone),
                    "Go to the page about the best berry"
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
                    target: NtPath(String::from("/blog/1")),
                    "Go to first blog post"
                }
            }
            li {
                Link {
                    target: NtName("blog_post",vec![("blog_id", String::from("2"))], QNone),
                    "Go to second blog post"
                }
            }
            li {
                Link {
                    target: NtName("blog_post",vec![("blog_id", String::from("🎺"))], QNone),
                    "Go to trumpet blog post 🎺"
                }
            }
        }
    })
}

#[allow(non_snake_case)]
fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx).expect("called in router");

    let id = route.parameters.get("blog_id");
    let title = id
        .map(|id| format!("Blog Post: {id}"))
        .unwrap_or(String::from("Unknown Blog Post"));

    cx.render(rsx! {
        h2 { [title] }
        pre {
           "{id:#?}"
        }
        Link {
            target: NtPath(String::from("..")),
            "go to blog list"
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

#[allow(non_snake_case)]
fn PathDisplay(cx: Scope) -> Element {
    let route = use_route(&cx).expect("called in router");

    let path = &route.path;

    cx.render(rsx! {
        span {
            strong {"current path: "}
            "{path}"
        }
    })
}

#[allow(non_snake_case)]
fn RaspberryPage(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Raspberries are very tasty!" }
    })
}

#[allow(non_snake_case)]
fn StrawberryPage(cx: Scope) -> Element {
    cx.render(rsx! {
        span {
            class: "other",
            "Strawberries are good too!"
        }
    })
}
