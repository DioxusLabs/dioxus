use std::rc::Rc;

use dioxus::{prelude::*, web::HashHistory};

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().history(Rc::new(HashHistory::new(true))))
        .launch(|| {
            rsx! {
                Router::<Route> {}
            }
        })
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[redirect("/",|| Route::Other)]
    #[route("/other")]
    Other,
    #[route("/other/:id")]
    OtherId { id: String },
    #[route("/scroll?:page#:section")]
    Scroll { page: String, section: String },
    #[route("/elsewhere")]
    Elsewhere,
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
fn Other() -> Element {
    rsx! {
        div {
            id: "other",
            "Other"
        }

        Link {
            id: "other-id-link",
            to: Route::OtherId { id: "123".to_string() },
            "go to OtherId"
        }
    }
}

#[component]
fn OtherId(id: String) -> Element {
    rsx! {
        div {
            id: "other-id",
            "OtherId {id}"
        }
    }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div {
            id: "not-found",
            "NotFound {segments:?}"
        }
    }
}

#[component]
fn Scroll(page: String, section: String) -> Element {
    let navigator = navigator();
    let query_route = Route::Scroll {
        page: "2".to_string(),
        section: section.clone(),
    };
    let fragment_route = Route::Scroll {
        page: page.clone(),
        section: "section-target".to_string(),
    };
    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0;",
            button {
                id: "replace-query",
                onclick: move |_| {
                    navigator.replace(query_route.clone());
                },
                "Replace Query"
            }
            button {
                id: "replace-fragment",
                onclick: move |_| {
                    navigator.replace(fragment_route.clone());
                },
                "Replace Fragment"
            }
            button {
                id: "replace-path",
                onclick: move |_| {
                    navigator.replace(Route::Elsewhere);
                },
                "Replace Path"
            }
            button {
                id: "push-path",
                onclick: move |_| {
                    navigator.push(Route::Elsewhere);
                },
                "Push Path"
            }
        }
        div {
            id: "scroll",
            style: "position: relative; height: 5000px;",
            "Scroll {page}"
            div {
                id: "section-target",
                style: "position: absolute; top: 3000px;",
                "Section"
            }
        }
    }
}

#[component]
fn Elsewhere() -> Element {
    rsx! {
        div {
            id: "elsewhere",
            style: "height: 5000px;",
            "Elsewhere"
        }
    }
}
