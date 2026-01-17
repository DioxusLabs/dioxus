#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    // Make sure to set the url of the server where server functions are hosted - they aren't always at localhost
    #[cfg(not(feature = "server"))]
    dioxus::fullstack::set_server_url("http://127.0.0.1:8080");

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
                let data = get_server_data().await?;
                println!("Client received: {}", data);
                text.set(data.clone());
                post_server_data(data).await?;
                Ok(())
            },
            "Run a server function"
        }
        "Server said: {text}"
    }
}

#[post("/api/data")]
async fn post_server_data(data: String) -> ServerFnResult {
    println!("Server received: {}", data);
    Ok(())
}

#[get("/api/data")]
async fn get_server_data() -> ServerFnResult<String> {
    Ok("Hello from the server!".to_string())
}
