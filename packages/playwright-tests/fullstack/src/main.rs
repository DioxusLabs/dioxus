// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::fullstack::{commit_initial_chunk, Websocket};
use dioxus::{fullstack::WebSocketOptions, prelude::*};

fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        use dioxus::server::axum::{self, Extension};

        let cfg = dioxus::server::ServeConfig::builder().enable_out_of_order_streaming();
        let router = axum::Router::new()
            .serve_dioxus_application(cfg, app)
            .layer(Extension(1234u32));

        Ok(router)
    });
    #[cfg(not(feature = "server"))]
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 12345);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        Title { "hello axum! {count}" }
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
        DefaultServerFnCodec {}
        DocumentElements {}
        Assets {}
        WebSockets {}
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

#[component]
fn DefaultServerFnCodec() -> Element {
    let resource = use_server_future(|| get_server_data_empty_vec(Vec::new()))?;
    let empty_vec = resource.unwrap().unwrap();
    assert!(empty_vec.is_empty());

    rsx! {}
}

#[cfg(feature = "server")]
async fn assert_server_context_provided() {
    use dioxus::{fullstack::FullstackContext, server::axum::Extension};
    // Just make sure the server context is provided
    let Extension(id): Extension<u32> = FullstackContext::extract().await.unwrap();
    assert_eq!(id, 1234u32);
}

#[server]
async fn post_server_data(data: String) -> ServerFnResult {
    assert_server_context_provided().await;
    println!("Server received: {}", data);

    Ok(())
}

#[server]
async fn get_server_data() -> ServerFnResult<String> {
    assert_server_context_provided().await;
    Ok("Hello from the server!".to_string())
}

// Make sure the default codec work with empty data structures
// Regression test for https://github.com/DioxusLabs/dioxus/issues/2628
#[server]
async fn get_server_data_empty_vec(empty_vec: Vec<String>) -> ServerFnResult<Vec<String>> {
    assert_server_context_provided().await;
    assert!(empty_vec.is_empty());
    Ok(Vec::new())
}

#[server]
async fn server_error() -> ServerFnResult<String> {
    use dioxus_core::AnyhowContext;
    assert_server_context_provided().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Err(None.context("Server error occurred")?)
}

#[component]
fn Errors() -> Element {
    // Make the suspense boundary below happen during streaming
    use_hook(commit_initial_chunk);

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
    use_server_future(server_error)?.unwrap()?;
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

/// Make sure assets in the assets folder are served correctly and hashed assets are cached forever
#[component]
fn Assets() -> Element {
    #[used]
    static _ASSET: Asset = asset!("/assets/image.png");

    #[used]
    static _STATIC_NO_HASH: Asset = asset!(
        "/assets/image.png",
        AssetOptions::image().with_hash_suffix(false)
    );

    #[used]
    static _UNHASHED_FOLDER: Asset = asset!(
        "/assets/nested/",
        AssetOptions::folder().with_hash_suffix(false)
    );

    #[used]
    static _EMBEDDED_FOLDER: Asset = asset!("/assets/nested");

    rsx! {
        img {
            src: asset!("/assets/image.png"),
        }
        img {
            src: "{_EMBEDDED_FOLDER}/image.png",
        }
        img {
            src: "{_UNHASHED_FOLDER}/image.png",
        }
        img {
            src: "/assets/image.png",
        }
    }
}

/// This component tests websocket server functions
#[component]
fn WebSockets() -> Element {
    let mut received = use_signal(String::new);

    use_future(move || async move {
        let socket = echo_ws(WebSocketOptions::default()).await.unwrap();

        socket.send("hello world".to_string()).await.unwrap();

        while let Ok(msg) = socket.recv().await {
            received.write().push_str(&msg);
        }
    });

    rsx! {
        div {
            id: "websocket-div",
            "Received: {received}"
        }
    }
}

#[get("/api/echo_ws")]
async fn echo_ws(options: WebSocketOptions) -> Result<Websocket> {
    info!("Upgrading to websocket");

    Ok(options.on_upgrade(
        |mut tx: dioxus::fullstack::TypedWebsocket<String, String>| async move {
            while let Ok(msg) = tx.recv().await {
                let _ = tx.send(msg.to_ascii_uppercase()).await;
            }
        },
    ))
}
