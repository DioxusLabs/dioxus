use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_web::launch(App);
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            .fixed(
                "blog",
                Route::new(Blog as Component).nested(
                    Segment::default().index(BlogList as Component).catch_all(
                        ParameterRoute::new("post_id", BlogPost as Component).name(BlogPost),
                    ),
                ),
            )
            .fixed("myblog", "/blog")
            .fallback(PageNotFound as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            NavBar {}
            Outlet {}
        }
    })
}

fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul {
                li { Link { target: (RootIndex, []), "Home" } }
                li { Link { target: "/blog", "Blog" } }
            }
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to the Dioxus Blog!" }
    })
}

fn Blog(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Blog" }
        Outlet {}
    })
}

fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Choose a post" }
        ul {
            li { Link {
                target: (BlogPost, [("post_id", String::from("1"))]),
                "Read the first blog post"
            } }
            li { Link {
                target: (BlogPost, [("post_id", String::from("2"))]),
                "Read the second blog post"
            } }
        }
    })
}

fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();

    let post_id = route.parameters.get("post_id");
    let post = post_id
        .map(|id| id.to_string())
        .unwrap_or(String::from("unknown"));

    cx.render(rsx! {
        h2 { "Blog Post: {post}"}
    })
}

fn PageNotFound(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
    })
}
