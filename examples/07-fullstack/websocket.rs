//! This example showcases the built-in websocket functionality in Dioxus Fullstack.
//!
//! We can create a new websocket endpoint that takes the WebSocketOptions as a body and returns
//! a `Websocket` instance that the client uses to communicate with the server.
//!
//! The `Websocket` type is generic over the message types and the encoding used to serialize the messages.
//!
//! By default, we use `JsonEncoding`, but in this example, we use `CborEncoding` to demonstrate that
//! binary encodings also work.
//!
//! The `use_websocket` hook wraps the `Websocket` instance and provides a reactive interface to the
//! state of the connection, as well as methods to send and receive messages.
//!
//! Because the websocket is generic over the message types, calls to `.recv()` and `.send()` are
//! strongly typed, making it easy to send and receive messages without having to manually
//! serialize and deserialize them.

use dioxus::{fullstack::CborEncoding, prelude::*};
use dioxus_fullstack::{WebSocketOptions, Websocket, use_websocket};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn main() {
    dioxus::launch(app);
}

// Static connection counter for connection limit enforcement
static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
const MAX_CONNECTIONS: usize = 100;
const MAX_MESSAGE_SIZE: usize = 65536; // 64KB limit per message

fn app() -> Element {
    // Track the messages we've received from the server.
    let mut messages = use_signal(std::vec::Vec::new);

    // The `use_websocket` wraps the `WebSocket` connection and provides a reactive handle to easily
    // send and receive messages and track the connection state.
    //
    // We can customize the websocket connection with the `WebSocketOptions` struct, allowing us to
    // set things like custom headers, protocols, reconnection strategies, etc.
    let mut socket = use_websocket(|| uppercase_ws("John Doe".into(), 30, WebSocketOptions::new()));

    // Calling `.recv()` automatically waits for the connection to be established and deserializes
    // messages as they arrive.
    use_future(move || async move {
        while let Ok(msg) = socket.recv().await {
            messages.push(msg);
        }
    });

    rsx! {
        h1 { "WebSocket Example" }
        p { "Type a message and see it echoed back in uppercase!" }
        p { "Connection status: {socket.status():?}" }
        input {
            placeholder: "Type a message",
            oninput: move |e| async move { _ = socket.send(ClientEvent::TextInput(e.value())).await; },
        }
        button { onclick: move |_| messages.clear(), "Clear messages" }
        for message in messages.read().iter().rev() {
            pre { "{message:?}" }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientEvent {
    TextInput(String),
}

#[derive(Serialize, Deserialize, Debug)]
enum ServerEvent {
    Uppercase(String),
}

#[get("/api/uppercase_ws?name&age&token")]
async fn uppercase_ws(
    name: String,
    age: i32,
    token: Option<String>,
    options: WebSocketOptions,
) -> Result<Websocket<ClientEvent, ServerEvent, CborEncoding>> {
    // Authentication check: verify token (simplified for example)
    if token.as_deref() != Some("valid_token") {
        return Err(anyhow::anyhow!("Unauthorized: Invalid or missing authentication token"));
    }

    // Connection limit check
    if ACTIVE_CONNECTIONS.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
        return Err(anyhow::anyhow!("Connection limit reached"));
    }

    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);

    Ok(options.on_upgrade(move |mut socket| async move {
        // Ensure connection is decremented on drop
        let _guard = ConnectionGuard;

        // send back a greeting message
        _ = socket
            .send(ServerEvent::Uppercase(format!(
                "First message from server: Hello, {}! You are {} years old.",
                name, age
            )))
            .await;

        // Rate limiting: track message count per time window
        let mut message_count = 0u32;
        let mut last_reset = std::time::Instant::now();
        const RATE_LIMIT: u32 = 100; // messages per second
        const RATE_WINDOW: std::time::Duration = std::time::Duration::from_secs(1);

        // Loop and echo back uppercase messages with size and rate checks
        while let Ok(ClientEvent::TextInput(next)) = socket.recv().await {
            // Message size check
            if next.len() > MAX_MESSAGE_SIZE {
                let _ = socket
                    .send(ServerEvent::Uppercase(
                        "Error: Message too large".to_string(),
                    ))
                    .await;
                continue;
            }

            // Rate limiting check
            if last_reset.elapsed() > RATE_WINDOW {
                message_count = 0;
                last_reset = std::time::Instant::now();
            }

            if message_count >= RATE_LIMIT {
                let _ = socket
                    .send(ServerEvent::Uppercase(
                        "Error: Rate limit exceeded".to_string(),
                    ))
                    .await;
                continue;
            }

            message_count += 1;
            _ = socket
                .send(ServerEvent::Uppercase(next.to_uppercase()))
                .await;
        }
    }))
}

/// Guard to ensure ACTIVE_CONNECTIONS is decremented when connection closes
struct ConnectionGuard;

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    }
}
