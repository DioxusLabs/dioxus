use std::sync::{Arc, RwLockReadGuard};

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use log::info;

struct BlogPostName;
struct Raspberry;

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
                Route::new(Blog as Component).nested(
                    Segment::default()
                        .index(BlogWelcome as Component)
                        .parameter(
                            ParameterRoute::new("blog_id", BlogPost as Component)
                                .name(BlogPostName),
                        ),
                ),
            )
            .fixed(
                "raspberry",
                Route::new(RouteContent::Multi(
                    RaspberryPage,
                    vec![("other", StrawberryPage)],
                ))
                .name(Raspberry),
            )
            .fixed("the_best_berry", "/raspberry")
    });

    let update_fn = cx.use_hook(|| {
        Arc::new(|state: RwLockReadGuard<RouterState>| -> Option<_> {
            info!("current path: {}", state.path);
            None
        })
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
            routes: routes.clone(),
            fallback_named_navigation: NamedNavigationFallback,
            update_callback: update_fn.clone(),

            header {
                Link {
                    target: (RootIndex, [])
                    "go home"
                    active_class: "active",
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
                    target: ("nonexisting name", []),
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
                    target: (Raspberry, []),
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
                    target: "/blog/1/",
                    "Go to first blog post"
                }
            }
            li {
                Link {
                    target: (BlogPostName, [("blog_id", String::from("2"))]),
                    "Go to second blog post"
                }
            }
            li {
                Link {
                    target: (BlogPostName, [("blog_id", String::from("🎺"))]),
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
