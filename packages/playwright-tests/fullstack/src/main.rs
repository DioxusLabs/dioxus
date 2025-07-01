// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::prelude::{
    server_fn::{codec::JsonEncoding, BoxedStream, Websocket},
    *,
};
use futures::{channel::mpsc, SinkExt, StreamExt};

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
    let FromContext(i): FromContext<u32> = extract().await.unwrap();
    assert_eq!(i, 1234u32);
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> ServerFnResult {
    assert_server_context_provided().await;
    println!("Server received: {}", data);

    Ok(())
}

#[server(GetServerData)]
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
    assert_server_context_provided().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Err(ServerFnError::new("the server threw an error!"))
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
    static _OTHER_ASSET: Asset = asset!("/assets/nested");
    rsx! {
        img {
            src: asset!("/assets/image.png"),
        }
        img {
            src: "/assets/image.png",
        }
        img {
            src: "/assets/nested/image.png",
        }
    }
}

#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
async fn echo_ws(
    input: BoxedStream<String, ServerFnError>,
) -> ServerFnResult<BoxedStream<String, ServerFnError>> {
    let mut input = input;

    let (mut tx, rx) = mpsc::channel(1);

    tokio::spawn(async move {
        while let Some(msg) = input.next().await {
            let _ = tx.send(msg.map(|msg| msg.to_ascii_uppercase())).await;
        }
    });

    Ok(rx.into())
}

/// This component tests websocket server functions
#[component]
fn WebSockets() -> Element {
    let mut received = use_signal(String::new);
    use_future(move || async move {
        let (mut tx, rx) = mpsc::channel(1);
        let mut receiver = echo_ws(rx.into()).await.unwrap();
        tx.send(Ok("hello world".to_string())).await.unwrap();
        while let Some(Ok(msg)) = receiver.next().await {
            println!("Received: {}", msg);
            received.set(msg);
        }
    });

    rsx! {
        div {
            id: "websocket-div",
            "Received: {received}"
        }
    }
}
