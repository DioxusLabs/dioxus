//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use anyhow::Context;
use dioxus::prelude::*;
use dioxus::{fullstack::Websocket, logger::tracing};
use reqwest::StatusCode;

fn main() {
    dioxus::launch(|| {
        let mut count = use_signal(|| 0);
        let mut text = use_signal(|| "...".to_string());
        let server_future = use_server_future(get_server_data)?;

        rsx! {
            document::Link { href: asset!("/assets/hello.css"), rel: "stylesheet" }
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button {
                onclick: move |_| async move {
                    let data = get_server_data().await?;
                    println!("Client received: {}", data);
                    text.set(data.clone().to_string());
                    let err = post_server_data(data.to_string()).await;
                    // get_server_data2().await;
                    Ok(())
                },
                "Run a server function!"
            }
            "Server said: {text}"
        }
    });
}

#[post("/api/data")]
async fn post_server_data(data: String) -> Result<(), StatusCode> {
    println!("Server received: {}", data);
    Ok(())
}

#[get("/api/data")]
async fn get_server_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.json().await?)
}

// #[get("/api/ws")]
// async fn ws_endpoint(ws: String) -> Result<Websocket<String, String>> {
//     todo!()
// }

// #[get("/api/data")]
// async fn get_server_data2() -> axum::extract::Json<i32> {
//     axum::extract::Json(123)
// }

// new rules
//
// - only Result<T, E: From<ServerFnError>> is allowed as the return type.
// - all arguments must be (de)serializable with serde *OR* a single argument that implements IntoRequest (and thus from request)
// - extra "fromrequestparts" things must be added in the attr args
//
// this forces every endpoint to be usable from the client
