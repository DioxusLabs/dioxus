//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

#![allow(non_snake_case, unused)]
use dioxus::logger::tracing;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
                    // post_server_data(data).await?;
                    Ok(())
                },
                "Run a server function!"
            }
            "Server said: {text}"
        }
    });
}

#[post("/api/data")]
async fn post_server_data(data: String) -> Result<()> {
    println!("Server received: {}", data);
    Ok(())
}

#[get("/api/data")]
async fn get_server_data() -> Result<serde_json::Value> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.json().await?)
}
