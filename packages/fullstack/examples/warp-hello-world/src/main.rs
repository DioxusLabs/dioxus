//! Run with:
//!
//! ```sh
//! dx build --features web
//! cargo run --features ssr
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_fullstack::{launch, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Props, PartialEq, Debug, Default, Serialize, Deserialize, Clone)]
struct AppProps {
    count: i32,
}

fn app(cx: Scope<AppProps>) -> Element {
    render! {
        Child {}
    }
}

fn Child(cx: Scope) -> Element {
    let state = use_server_future(cx, (), |()| async move {
        loop {
            if let Ok(res) = get_server_data().await {
                break res;
            }
        }
    })?
    .value();

    let mut count = use_state(cx, || 0);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        div {
            "Server state: {state}"
        }
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
            "Run a server function!"
        }
        "Server said: {text}"
    })
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
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    #[cfg(feature = "ssr")]
    simple_logger::SimpleLogger::new().init().unwrap();

    launch!(@([127, 0, 0, 1], 8080), app, {
        serve_cfg: ServeConfigBuilder::new(app, AppProps { count: 0 }),
    });
}
