use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_web::launch(App);
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration::default(),
        &|| {
            Segment::content(comp(Home))
                .fixed("blog", Route::content(comp(Blog)).nested(
                    Segment::content(comp(BlogList)).catch_all(
                        ParameterRoute::content::<PostId>(comp(BlogPost))
                            .name::<BlogPostName>()
                    )
                ))
                .fixed("myblog", "/blog") // this is new
                .fallback(comp(PageNotFound))
        }
    );

    cx.render(rsx! {
        NavBar {}
        Outlet {}
    })
}

fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul {
                li { Link { target: named::<RootIndex>(), "Home" } }
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
                target: named::<BlogPostName>().parameter::<PostId>("1"),
                "Read the first blog post"
            } }
            li { Link {
                target: named::<BlogPostName>().parameter::<PostId>("2"),
                "Read the second blog post"
            } }
        }
    })
}

struct PostId;
struct BlogPostName;
fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();

    let post_id = route.parameter::<PostId>();
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
