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
        let mut dog_data = use_action(move |()| get_dog_data());
        let mut ip_data = use_action(move |()| get_ip_data());

        rsx! {
            Stylesheet { href: asset!("/assets/hello.css")  }
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button { onclick: move |_| { dog_data.dispatch(()); }, "Fetch dog data" }
            button { onclick: move |_| { ip_data.dispatch(()); }, "Fetch IP data" }
            button {
                onclick: move |_| {
                    ip_data.reset();
                    dog_data.reset();
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

#[get("/api/ws")]
async fn ws_endpoint(ws: String) -> Result<Websocket<String, String>> {
    todo!()
}
