#![allow(non_snake_case)]
use dioxus::prelude::*;

/// Run with `cargo run --features desktop`
#[cfg(feature = "desktop")]
fn main() {
    // Set the url of the server where server functions are hosted.
    dioxus::fullstack::prelude::server_fn::client::set_server_url("http://127.0.0.1:8080");

    // And then launch the app
    dioxus::prelude::launch_desktop(app);
}

/// Run with `cargo run --features server`
#[cfg(not(feature = "desktop"))]
#[tokio::main]
async fn main() {
    use server_fn::axum::register_explicit;

    let listener = tokio::net::TcpListener::bind("127.0.0.01:8080")
        .await
        .unwrap();

    register_explicit::<PostServerData>();
    register_explicit::<GetServerData>();

    axum::serve(
        listener,
        axum::Router::new()
            .register_server_functions()
            .into_make_service(),
    )
    .await
    .unwrap();
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
