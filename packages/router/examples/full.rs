use dioxus::{prelude::*, router::PATH_FOR_NAMED_NAVIGATION_FAILURE};

fn main() {
    env_logger::init();
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
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
            .fixed(
                "the_best_berry",
                Route::new(RcRedirect(NtName("raspberry", vec![], QNone))).name("best_berry"),
            )
            .fixed(
                PATH_FOR_NAMED_NAVIGATION_FAILURE,
                Route::new(RcComponent(NamedNavigationFallback)),
            )
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
