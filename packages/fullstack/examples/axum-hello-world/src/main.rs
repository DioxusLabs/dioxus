//! Run with:
//!
//! ```sh
//! dioxus build --features web
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
    let mut count = use_state(cx, || cx.props.count);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button {
            onclick: move |_| {
                to_owned![text];
                let sc = cx.sc();
                async move {
                    if let Ok(data) = get_server_data().await {
                        println!("Client received: {}", data);
                        text.set(data.clone());
                        post_server_data(sc, data).await.unwrap();
                    }
                }
            },
            "Run a server function!"
        }
        "Server said: {text}"
    })
}

#[server(PostServerData)]
async fn post_server_data(cx: DioxusServerContext, data: String) -> Result<(), ServerFnError> {
    // The server context contains information about the current request and allows you to modify the response.
    cx.response_headers_mut()
        .insert("Set-Cookie", "foo=bar".parse().unwrap());
    println!("Server received: {}", data);
    println!("Request parts are {:?}", cx.request_parts());

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}

fn main() {
    launch!(@([127, 0, 0, 1], 8080), app, {
        serve_cfg: ServeConfigBuilder::new(app, (AppProps { count: 0 })),
        incremental: IncrementalRendererConfig::default().invalidate_after(std::time::Duration::from_secs(120)),
    });
}
