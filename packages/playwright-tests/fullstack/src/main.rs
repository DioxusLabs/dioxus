// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::{prelude::*, CapturedError};

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only! {
            dioxus::fullstack::ServeConfig::builder().enable_out_of_order_streaming()
        })
        .with_context(1234u32)
        .launch(app);
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
        OnMounted {}
        DocumentElements {}
    }
}

#[component]
fn OnMounted() -> Element {
    let mut mounted_triggered_count = use_signal(|| 0);
    rsx! {
        div {
            class: "onmounted-div",
            onmounted: move |_| {
                mounted_triggered_count += 1;
            },
            "onmounted was called {mounted_triggered_count} times"
        }
    }
}

#[cfg(feature = "server")]
async fn assert_server_context_provided() {
    let FromContext(i): FromContext<u32> = extract().await.unwrap();
    assert_eq!(i, 1234u32);
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    assert_server_context_provided().await;
    println!("Server received: {}", data);

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    assert_server_context_provided().await;
    Ok("Hello from the server!".to_string())
}

#[server]
async fn server_error() -> Result<String, ServerFnError> {
    assert_server_context_provided().await;
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

/// This component tests the document::* elements pre-rendered on the server
#[component]
fn DocumentElements() -> Element {
    rsx! {
        document::Meta { id: "meta-head", name: "testing", data: "dioxus-meta-element" }
        document::Link {
            id: "link-head",
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css?family=Roboto+Mono"
        }
        document::Stylesheet { id: "stylesheet-head", href: "https://fonts.googleapis.com/css?family=Roboto:300,300italic,700,700italic" }
        document::Script { id: "script-head", async: true, "console.log('hello world');" }
        document::Style { id: "style-head", "body {{ font-family: 'Roboto'; }}" }
    }
}
