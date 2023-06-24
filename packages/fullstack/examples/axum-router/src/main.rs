//! Run with:
//!
//! ```sh
//! dioxus build --features web
//! cargo run --features ssr
//! ```

#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    launch_router!(@([127, 0, 0, 1], 8080), Route, {
        incremental,
    });
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

#[inline_props]
fn Blog(cx: Scope, id: i32) -> Element {
    render! {
        Link { target: Route::Home {}, "Go to counter" }
        table {
            tbody {
                for _ in 0..*id {
                    tr {
                        for _ in 0..*id {
                            td { "hello world!" }
                        }
                    }
                }
            }
        }
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        Link {
            target: Route::Blog {
                id: *count.get()
            },
            "Go to blog"
        }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button {
                onclick: move |_| {
                    to_owned![text];
                    async move {
                        if let Ok(data) = get_server_data().await {
                            println!("Client received: {}", data);
                            text.set(data.clone());
                            post_server_data(data).await.unwrap();
                        }
                    }
                },
                "Run server function!"
            }
            "Server said: {text}"
        }
    })
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    println!("Server received: {}", data);

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}
