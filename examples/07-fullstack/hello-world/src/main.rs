//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use anyhow::Context;
use dioxus::fullstack::{Json, Websocket};
use dioxus::prelude::*;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(|| {
        let mut count = use_signal(|| 0);
        let mut dog_data = use_action(move |()| get_dog_data());
        let mut ip_data = use_action(move |()| get_ip_data());
        let mut custom_data = use_action(move |()| async move {
            info!("Fetching custom encoded data");
            get_custom_encoding(Json(serde_json::json!({
                "example": "data",
                "number": 123,
                "array": [1, 2, 3],
            })))
            .await
        });
        let mut error_data = use_action(move |()| get_throws_error());
        let mut typed_error_data = use_action(move |()| async move {
            let result = get_throws_typed_error().await;
            info!("Fetching typed error data: {result:#?}");
            result
        });

        rsx! {
            Stylesheet { href: asset!("/assets/hello.css")  }
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button { onclick: move |_| { dog_data.dispatch(()); }, "Fetch dog data" }
            button { onclick: move |_| { ip_data.dispatch(()); }, "Fetch IP data" }
            button { onclick: move |_| { custom_data.dispatch(()); }, "Fetch custom encoded data" }
            button { onclick: move |_| { error_data.dispatch(()); }, "Fetch error data" }
            button { onclick: move |_| { typed_error_data.dispatch(()); }, "Fetch typed error data" }
            button {
                onclick: move |_| {
                    ip_data.reset();
                    dog_data.reset();
                    custom_data.reset();
                },
                "Clear data"
            }
            div {
                pre {
                    "Dog data: "
                    if dog_data.is_pending() { "(loading...) " }
                    "{dog_data.value():#?}"
                }
            }
            div {
                pre {
                    "IP data: "
                    if ip_data.is_pending() { "(loading...) " }
                    "{ip_data.value():#?}"
                }
            }
            div {
                pre {
                    "Custom encoded data: "
                    if custom_data.is_pending() { "(loading...) " }
                    "{custom_data.value():#?}"
                }
            }
            div {
                pre {
                    "Error data: "
                    if error_data.is_pending() { "(loading...) " }
                    "{error_data.result():#?}"
                }
            }
            div {
                pre {
                    "Typed error data: "
                    if typed_error_data.is_pending() { "(loading...) " }
                    "{typed_error_data.result():#?}"
                }
            }
        }
    });
}

#[post("/api/data")]
async fn post_server_data(data: String) -> Result<(), StatusCode> {
    println!("Server received: {}", data);
    Ok(())
}

#[get("/api/data")]
async fn get_ip_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.json().await?)
}

#[get("/api/dog-data")]
async fn get_dog_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://dog.ceo/api/breeds/image/random")
        .await?
        .json()
        .await?)
}

#[post("/api/custom-encoding")]
async fn get_custom_encoding(takes: Json<serde_json::Value>) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "message": "This response was encoded with a custom encoder!",
        "success": true,
        "you sent": takes.0,
    }))
}

#[get("/api/untyped-error")]
async fn get_throws_error() -> Result<()> {
    Err(anyhow::anyhow!("This is an example error").context("Additional context"))
}

#[get("/api/typed-error")]
async fn get_throws_typed_error() -> Result<(), MyCustomError> {
    Err(MyCustomError::BadRequest {
        details: "Invalid input".into(),
    })
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
enum MyCustomError {
    #[error("bad request")]
    BadRequest { details: String },
    #[error("not found")]
    NotFound,
    #[error("internal server error: {0}")]
    ServerFnError(#[from] ServerFnError),
}

#[post("/api/ws")]
async fn ws_endpoint(a: i32) -> Result<Websocket<String, String>> {
    todo!()
}
