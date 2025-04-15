//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
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
                if let Ok(data) = get_server_data().await {
                    println!("Client received: {}", data);
                    text.set(data.clone());
                    post_server_data(data).await.unwrap();
                }

            },
            "Run a server function!"
        }
        button {
            onclick: move |_| async move {
                if let Ok(data) = say_hi().await {
                    text.set(data.clone());
                } else {
                    text.set("Error".to_string());
                }
            },
            "Say hi!"
        }
        "Server said: {text}"
    }
}

#[server]
async fn say_hi() -> Result<String, ServerFnError> {
    Ok("hello 123".to_string())
}

#[server]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    println!("Server recesdasdasdasdasasdaasdsdasdisdasdvedasd: {}", data);

    Ok(())
}

#[server]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}
