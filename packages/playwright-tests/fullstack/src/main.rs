// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::{prelude::*, CapturedError};

fn main() {
    LaunchBuilder::fullstack().launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 12345);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        document::Title { "hello axum! {count}" }
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
        div {
            id: "errors",
            Errors {}
        }
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

#[server]
async fn server_error() -> Result<String, ServerFnError> {
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Err(ServerFnError::new("the server threw an error!"))
}

#[component]
fn Errors() -> Element {
    rsx! {
        // This is a tricky case for suspense https://github.com/DioxusLabs/dioxus/issues/2570
        // Root suspense boundary is already resolved when the inner suspense boundary throws an error.
        // We need to throw the error from the inner suspense boundary on the server to the hydrated
        // suspense boundary on the client
        ErrorBoundary {
            handle_error: |_| rsx! {
                "Hmm, something went wrong."
            },
            SuspenseBoundary {
                fallback: |_: SuspenseContext| rsx! {
                    div {
                        "Loading..."
                    }
                },
                ThrowsError {}
            }
        }
    }
}

#[component]
pub fn ThrowsError() -> Element {
    use_server_future(server_error)?
        .unwrap()
        .map_err(|err| RenderError::Aborted(CapturedError::from_display(err)))?;
    rsx! {
        "success"
    }
}
