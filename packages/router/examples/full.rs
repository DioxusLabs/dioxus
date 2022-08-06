use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    env_logger::init();
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            .fixed(
                "blog",
                Route::new(RcComponent(Blog)).nested(
                    Segment::default()
                        .index(RcComponent(BlogWelcome))
                        .parameter(
                            ParameterRoute::new("blog_id", RcComponent(BlogPost)).name("blog_post"),
                        ),
                ),
            )
            .fixed(
                "raspberry",
                Route::new(RcMulti(RaspberryPage, vec![("other", StrawberryPage)]))
                    .name("raspberry"),
            )
            .fixed("the_best_berry", "/raspberry")
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
            routes: routes.clone(),
            fallback_named_navigation: NamedNavigationFallback,

            header {
                Link {
                    target: NamedTarget("", vec![], None)
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
                    target: "/blog",
                    "go to blog"
                }
            }
            li {
                Link {
                    target: NamedTarget("nonexisting name", vec![], None),
                    "trigger a named navigation error"
                }
            }
            li {
                Link {
                    target: "https://dioxuslabs.com/",
                    "Go to an external website"
                }
            }
            li {
                Link {
                    target: NamedTarget("raspberry", vec![], None),
                    "Go to the page about raspberries"
                }
            }
            li {
                Link {
                    target: "/the_best_berry",
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
                    target: "/blog/1",
                    "Go to first blog post"
                }
            }
            li {
                Link {
                    target: NamedTarget("blog_post",vec![("blog_id", String::from("2"))], None),
                    "Go to second blog post"
                }
            }
            li {
                Link {
                    target: NamedTarget("blog_post",vec![("blog_id", String::from("🎺"))], None),
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
            target: "..",
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
