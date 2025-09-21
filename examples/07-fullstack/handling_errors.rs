//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use anyhow::Context;
use dioxus::fullstack::{FileUpload, Json, Websocket};
use dioxus::prelude::*;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(|| {
        let mut count = use_signal(|| 0);
        let mut dog_data = use_action(move |()| get_dog_data());
        let mut dog_data_err = use_action(move |()| get_dog_data_err());
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
        let mut throws_ok_data = use_action(move |()| get_throws_ok());

        rsx! {
            Stylesheet { href: asset!("/assets/hello.css")  }
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button { onclick: move |_| { dog_data.call(()); }, "Fetch dog data" }
            button { onclick: move |_| { ip_data.call(()); }, "Fetch IP data" }
            button { onclick: move |_| { custom_data.call(()); }, "Fetch custom encoded data" }
            button { onclick: move |_| { error_data.call(()); }, "Fetch error data" }
            button { onclick: move |_| { typed_error_data.call(()); }, "Fetch typed error data" }
            button { onclick: move |_| { dog_data_err.call(()); }, "Fetch dog error data" }
            button { onclick: move |_| { throws_ok_data.call(()); }, "Fetch throws ok data" }
            button {
                onclick: move |_| {
                    ip_data.reset();
                    dog_data.reset();
                    custom_data.reset();
                    error_data.reset();
                    typed_error_data.reset();
                    dog_data_err.reset();
                    throws_ok_data.reset();
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
            div {
                pre {
                    "Dog error data: "
                    if dog_data_err.is_pending() { "(loading...) " }
                    "{dog_data_err.result():#?}"
                }
            }
            div {
                pre {
                    "Throws ok data: "
                    if throws_ok_data.is_pending() { "(loading...) " }
                    "{throws_ok_data.result():#?}"
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

#[get("/api/ip-data")]
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

#[get("/api/dog-data-err")]
async fn get_dog_data_err() -> Result<serde_json::Value> {
    Ok(
        reqwest::get("https://dog.ceo/api/breed/NOT_A_REAL_DOG/images")
            .await?
            .json()
            .await?,
    )
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
    Err(anyhow::anyhow!("This is an example error"))
}

#[get("/api/throws-ok")]
async fn get_throws_ok() -> Result<()> {
    Ok(())
}

#[get("/api/typed-error")]
async fn get_throws_typed_error() -> Result<(), MyCustomError> {
    Err(MyCustomError::BadRequest {
        custom_name: "Invalid input".into(),
    })
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
enum MyCustomError {
    #[error("bad request")]
    BadRequest { custom_name: String },
    #[error("not found")]
    NotFound,
    #[error("internal server error: {0}")]
    ServerFnError(#[from] ServerFnError),
}

#[post("/api/ws")]
async fn ws_endpoint(a: i32) -> Result<Websocket<String, String>> {
    todo!()
}

/// We can extract the path arg and return anything thats IntoResponse
#[get("/upload/image/")]
async fn streaming_file(body: FileUpload) -> Result<Json<i32>> {
    todo!()
}

/// We can extract the path arg and return anything thats IntoResponse
#[get("/upload/image-args/?name&size&ftype")]
async fn streaming_file_args(
    name: String,
    size: usize,
    ftype: String,
    body: FileUpload,
) -> Result<Json<i32>> {
    todo!()
}
