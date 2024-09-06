#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    // Set the url of the server where server functions are hosted.
    #[cfg(not(feature = "server"))]
    dioxus::fullstack::prelude::server_fn::client::set_server_url("http://127.0.0.1:8080");
    dioxus::launch(app);
}

pub fn app() -> Element {
    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
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
            "Run a server function"
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
