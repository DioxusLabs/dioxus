use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    launch_desktop(Route::Home {});
}

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[nest("/blog")]
            #[layout(Blog)]
                #[route("/")]
                BlogList {},
                #[route("/:name")]
                BlogPost { name: String },
            #[end_layout]
        #[end_nest]
    #[end_layout]
    #[nest("/myblog")]
        #[redirect("/", || Route::BlogList {})]
        #[redirect("/:name", |name: String| Route::BlogPost { name })]
    #[end_nest]
    #[route("/:..route")]
    PageNotFound {
        route: Vec<String>,
    },
}

#[component]
fn NavBar() -> Element {
    rsx! {
        nav {
            ul {
                li {
                    Link { to: Route::Home {}, "Home" }
                }
                li {
                    Link { to: Route::BlogList {}, "Blog" }
                }
            }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! { h1 { "Welcome to the Dioxus Blog!" } }
}

#[component]
fn Blog() -> Element {
    rsx! {
        h1 { "Blog" }
        Outlet::<Route> {}
    }
}

#[component]
fn BlogList() -> Element {
    rsx! {
        h2 { "Choose a post" }
        ul {
            li {
                Link {
                    to: Route::BlogPost {
                        name: "Blog post 1".into(),
                    },
                    "Read the first blog post"
                }
            }
            li {
                Link {
                    to: Route::BlogPost {
                        name: "Blog post 2".into(),
                    },
                    "Read the second blog post"
                }
            }
        }
    }
}

#[component]
fn BlogPost(name: String) -> Element {
    rsx! { h2 { "Blog Post: {name}" } }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre { color: "red", "log:\nattemped to navigate to: {route:?}" }
    }
}
