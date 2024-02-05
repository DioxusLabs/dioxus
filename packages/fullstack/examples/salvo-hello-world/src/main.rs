//! Run with:
//!
//! ```sh
//! dx serve --platform fullstack
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_fullstack::{launch, prelude::*};
use serde::{Deserialize, Serialize};

fn app() -> Element {
    let state = use_server_future(move || async move { get_server_data().await.unwrap() })?;

    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        div { "Server state: {state.value().unwrap()}" }
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
        "Server said: {text}"
    }
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    println!("Server received: {}", data);

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

fn main() {
    #[cfg(feature = "web")]
    tracing_wasm::set_as_global_default();

    #[cfg(feature = "server")]
    tracing_subscriber::fmt::init();

    LaunchBuilder::fullstack().launch(app);
}
