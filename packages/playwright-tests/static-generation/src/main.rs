// This test is used by playwright configured in the root of the repo
// Tests:
// - Static Generation
// - Simple Suspense
// - Hydration

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::static_generation().launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 12345);
    let server_data = use_server_future(get_server_data)?;

    rsx! {
        h1 { "hello axum! {count}" }
        button { class: "increment-button", onclick: move |_| count += 1, "Increment" }
        "Server said: {server_data().unwrap():?}"
    }
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    Ok("Hello from the server!".to_string())
}
