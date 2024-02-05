// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::fullstack()
        .with_cfg(ssr! {
            dioxus::fullstack::Config::default().addr(std::net::SocketAddr::from(([127, 0, 0, 1], 3333)))
        })
        .launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 12345);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        h1 { "hello axum! {count}" }
        button { class: "increment-button", onclick: move |_| count += 1, "Increment" }
        button {
            class: "server-button",
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
    Ok("Hello from the server!".to_string())
}
