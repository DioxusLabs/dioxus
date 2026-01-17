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

fn main() {
    dioxus::launch(app);
}

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

#[get("/api/uppercase_ws?name&age")]
async fn uppercase_ws(
    name: String,
    age: i32,
    options: WebSocketOptions,
) -> Result<Websocket<ClientEvent, ServerEvent, CborEncoding>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        // send back a greeting message
        _ = socket
            .send(ServerEvent::Uppercase(format!(
                "First message from server: Hello, {}! You are {} years old.",
                name, age
            )))
            .await;

        // Loop and echo back uppercase messages
        while let Ok(ClientEvent::TextInput(next)) = socket.recv().await {
            _ = socket.send(ServerEvent::Uppercase(next)).await;
        }
    }))
}
