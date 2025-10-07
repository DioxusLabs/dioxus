// This test is used by playwright configured in the root of the repo
// Tests:
// - 200 Routes
// - 404 Routes
// - 500 Routes

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only! {
            dioxus::server::ServeConfig::builder().enable_out_of_order_streaming()
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

    #[route("/async-error")]
    ThrowsAsyncError,

    #[route("/can-go-back")]
    HydrateCanGoBack,
}

#[component]
fn Blog(id: i32) -> Element {
    let route: Route = use_route();
    assert_eq!(route, Route::Blog { id });

    rsx! {
        Link { to: Route::Home {}, "Go home" }
        "id: {id}"
    }
}

#[component]
fn ThrowsError() -> Element {
    dioxus::core::bail!("This route tests uncaught errors in the server",)
}

#[component]
fn ThrowsAsyncError() -> Element {
    #[server]
    async fn error_after_delay() -> ServerFnResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Err(ServerFnError::new("Async error from a server function"))
    }

    use_server_future(error_after_delay)?().unwrap()?;
    rsx! {
        "Hello, world!"
    }
}

#[component]
fn Home() -> Element {
    let route: Route = use_route();
    assert_eq!(route, Route::Home);

    rsx! {
        "Home"
        Link { to: Route::Blog { id: 1 }, "Go to blog 1" }
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
