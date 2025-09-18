//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use anyhow::Context;
use dioxus::prelude::*;
use dioxus::{
    fullstack::{Json, Websocket},
    logger::tracing,
};
use reqwest::StatusCode;

fn main() {
    dioxus::launch(|| {
        let mut count = use_signal(|| 0);
        let mut dog_data = use_action(move |()| get_dog_data());
        let mut ip_data = use_action(move |()| get_ip_data());
        let mut custom_data = use_action(move |()| get_custom_encoding());

        rsx! {
            Stylesheet { href: asset!("/assets/hello.css")  }
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button { onclick: move |_| { dog_data.dispatch(()); }, "Fetch dog data" }
            button { onclick: move |_| { ip_data.dispatch(()); }, "Fetch IP data" }
            button { onclick: move |_| { custom_data.dispatch(()); }, "Fetch custom encoded data" }
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

#[get("/api/custom-encoding")]
async fn get_custom_encoding() -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "This response was encoded with a custom encoder!",
        "success": true,
    })))
}

#[get("/api/ws")]
async fn ws_endpoint(a: i32) -> Result<Websocket<String, String>> {
    todo!()
}
