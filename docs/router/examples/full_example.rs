use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_web::launch(App);
}

#[derive(Routable, Clone, Serialize, Deserialize)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[nest("/blog")]
        #[layout(Blog)]
            #[route("/")]
            BlogList {},
            #[route("/blog/:id")]
            BlogPost { id: String },
        #[end_layout]
    #[end_nest]
    #[redirect("/myblog", || Route::BlogList {})]
    #[route("/:...route")]
    PageNotFound {
        route: Vec<String>,
    },
}

fn App(cx: Scope) -> Element {
    render! {
        NavBar {}
        Router {}
    }
}

fn NavBar(cx: Scope) -> Element {
    render! {
        nav {
            ul {
                li { Link { target: Route::Home {}, "Home" } }
                li { Link { target: Route::BlogList {}, "Blog" } }
            }
        }
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}

#[inline_props]
fn Blog(cx: Scope) -> Element {
    render! {
        h1 { "Blog" }
        Outlet {}
    }
}

#[inline_props]
fn BlogList(cx: Scope) -> Element {
    render! {
        h2 { "Choose a post" }
        ul {
            li {
                Link {
                    target: Route::BlogPost { id: 1 },
                    "Read the first blog post"
                }
            }
            li {
                Link {
                    target: Route::BlogPost { id: 2 },
                    "Read the second blog post"
                }
            }
        }
    }
}

#[inline_props]
fn BlogPost(cx: Scope, id: usize) -> Element {
    render! {
        h2 { "Blog Post: {id}"}
    }
}

#[inline_props]
fn PageNotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre {
            color: "red",
            "log:\nattemped to navigate to: {route:?}"
        }
    }
}
