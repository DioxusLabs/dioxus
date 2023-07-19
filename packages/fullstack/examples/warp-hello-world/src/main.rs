//! Run with:
//!
//! ```sh
//! dx build --features web
//! cargo run --features ssr
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    launch!(@([127, 0, 0, 1], 8080), app, (AppProps { count: 5 }), {
        incremental: IncrementalRendererConfig::default().invalidate_after(std::time::Duration::from_secs(120)),
    });
}

#[derive(Props, PartialEq, Debug, Default, Serialize, Deserialize, Clone)]
struct AppProps {
    count: i32,
}

fn app(cx: Scope<AppProps>) -> Element {
    let mut count = use_state(cx, || cx.props.count);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
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
            "Run a server function"
        }
        "Server said: {text}"
    })
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    // The server context contains information about the current request and allows you to modify the response.
    let cx = server_context();
    println!("Server received: {}", data);
    println!("Request parts are {:?}", cx.request_parts());

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}
