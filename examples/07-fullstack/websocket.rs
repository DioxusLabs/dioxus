use dioxus::prelude::*;
use dioxus_fullstack::{WebSocketOptions, Websocket, use_websocket};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Track the messages we've received from the server.
    let mut messages = use_signal(std::vec::Vec::new);

    // The `use_websocket` wraps the `WebSocket` connection and provides a reactive handle to easily
    // send and receive messages and track the connection state.
    let mut socket = use_websocket(|| uppercase_ws("John Doe".into(), 30, WebSocketOptions::new()));

    // We can use the methods on the socket to send and receive messages.
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
            oninput: move |e| async move { socket.send(ClientEvent::TextInput(e.value())).await; },
        }
        button {
            onclick: move |_| messages.clear(),
            "Clear messages"
        }
        for message in messages.read().iter().rev() {
            pre { "{message:?}" }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum ClientEvent {
    TextInput(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum ServerEvent {
    Uppercase(String),
}

#[get("/api/uppercase_ws?name&age")]
async fn uppercase_ws(
    name: String,
    age: i32,
    options: WebSocketOptions,
) -> Result<Websocket<ClientEvent, ServerEvent>> {
    use axum::extract::ws::Message;
    use dioxus::fullstack::axum;

    Ok(options.on_upgrade(move |mut socket| async move {
        loop {
            let msg = socket.recv().await.unwrap().unwrap();
            match msg {
                ClientEvent::TextInput(next) => {
                    socket.send(ServerEvent::Uppercase(next)).await;
                }
            }
        }
    }))
}
