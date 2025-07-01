use std::rc::Rc;

use dioxus::{prelude::*, web::HashHistory};

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().history(Rc::new(HashHistory::new(false))))
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
