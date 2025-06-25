// This test is used by playwright configured in the root of the repo
// Tests:
// - 200 Routes
// - 404 Routes
// - 500 Routes

#![allow(non_snake_case)]
use dioxus::{prelude::*, CapturedError};

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only! {
            dioxus::fullstack::ServeConfig::builder().enable_out_of_order_streaming()
        })
        .launch(app);
}

fn app() -> Element {
    rsx! { Router::<Route> {} }
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Route {
    #[route("/")]
    Home,

    #[route("/blog/:id/")]
    Blog { id: i32 },

    #[route("/error")]
    ThrowsError,

    #[route("/can-go-back")]
    HydrateCanGoBack,
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go home" }
        "id: {id}"
    }
}

#[component]
fn ThrowsError() -> Element {
    return Err(RenderError::Aborted(CapturedError::from_display(
        "This route tests uncaught errors in the server",
    )));
}

#[component]
fn Home() -> Element {
    rsx! {
        "Home"
    }
}

#[component]
pub fn HydrateCanGoBack() -> Element {
    let navigator = use_navigator();
    let mut count = use_signal(|| 0);
    rsx! {
        header {
            class:"flex justify-start items-center app-bg-color-primary px-5 py-2 space-x-4",
            if navigator.can_go_back() {
                button  {
                    class: "app-button-circle item-navbar",
                    onclick: move |_| {
                        count += 1;
                    },
                    "{count}"
                },
            }
            else {
                div {
                    Link  {
                        class: "app-button-circle item-navbar",
                        to: Route::Home,
                        "Go to home"
                    },
                    button  {
                        class: "app-button-circle item-navbar",
                        onclick: move |_| {
                            count += 1;
                        },
                        "{count}"
                    },
                }
            }
        },
    }
}
