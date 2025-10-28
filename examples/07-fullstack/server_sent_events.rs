//! This example demonstrates server-sent events (SSE) using Dioxus Fullstack.
//!
//! Server-sent events allow the server to push updates to the client over a single HTTP connection.
//! This is useful for real-time updates, notifications, or any scenario where the server needs to
//! send data to the client without the client explicitly requesting it.
//!
//! SSE is a simpler alternative to WebSockets, not requiring a full-duplex, stateful connection with
//! the server. Instead, it uses a single long-lived HTTP connection to stream events from the server to the client.
//!
//! This means that SSE messages are stringly encoded, and thus binary data must be base64 encoded.
//! If you need to send binary data, consider using the `Streaming<T>` type instead, which lets
//! you send raw bytes over a streaming HTTP response with a custom encoding. You'd reach for SSE
//! when dealing with clients that might not support custom streaming protocols.
//!
//! Calling an SSE endpoint is as simple as calling any other server function. The return type of an
//! SSE endpoint is a `ServerEvents<T>` where `T` is the type of event you want to send to the client.
//!
//! On the client, the `ServerEvents<T>` type implements `Stream<Item = Result<T, ServerFnError>>`
//! so you can use it with async streams to get new events as they arrive.
//!
//! `T` must be serializable and deserializable, so anything that implements `Serialize` and `Deserialize`
//! can be used as an event type. Calls to `.recv()` will wait for the next event to arrive and
//! deserialize it into the correct type.

use dioxus::prelude::*;
use dioxus_fullstack::ServerEvents;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut events = use_signal(Vec::new);

    use_future(move || async move {
        // Call the SSE endpoint to get a stream of events
        let mut stream = listen_for_changes().await?;

        // And then poll it for new events, adding them to our signal
        while let Some(Ok(event)) = stream.recv().await {
            events.push(event);
        }

        dioxus::Ok(())
    });

    rsx! {
        h1 { "Events from server: " }
        for msg in events.read().iter().rev() {
            pre { "{msg:?}" }
        }
    }
}

/// We can send anything that's serializable as a server event - strings, numbers, structs, enums, etc.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
enum MyServerEvent {
    Yay { message: String },
    Nay { error: String },
}

/// Our SSE endpoint, when called, will return the ServerEvents handle which streams events to the client.
/// On the client, we can interact with this stream object to get new events as they arrive.
#[get("/api/sse")]
async fn listen_for_changes() -> Result<ServerEvents<MyServerEvent>> {
    use std::time::Duration;

    Ok(ServerEvents::new(|mut tx| async move {
        let mut count = 1;

        loop {
            // Create our serializable message
            let msg = if count % 5 == 0 {
                MyServerEvent::Nay {
                    error: "An error occurred".into(),
                }
            } else {
                MyServerEvent::Yay {
                    message: format!("Hello number {count}"),
                }
            };

            // Send the message to the client. If it errors, the client has disconnected
            if tx.send(msg).await.is_err() {
                // client disconnected, do some cleanup
                break;
            }

            count += 1;

            // Poll some data source here, subscribe to changes, maybe call an LLM?
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }))
}
