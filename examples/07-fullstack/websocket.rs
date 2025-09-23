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
            oninput: move |e| async move { socket.send(e.value()).await; },
        }
        for message in messages.iter() {
            pre { "{message}" }
        }
    }
}

#[get("/api/uppercase_ws?name&age")]
async fn uppercase_ws(name: String, age: i32, options: WebSocketOptions) -> Result<Websocket> {
    use axum::extract::ws::Message;
    use dioxus::fullstack::axum;

    Ok(options.on_upgrade(move |mut socket| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let json_message = serde_json::to_string("Hello from server").unwrap();
            if socket
                .send(Message::Text(json_message.into()))
                .await
                .is_err()
            {
                return;
            }
        }

        // let greeting = format!("Hello, {}! You are {} years old.", name, age);
        // if socket.send(Message::Text(greeting.into())).await.is_err() {
        //     return;
        // }

        // while let Some(Ok(msg)) = socket.recv().await {
        //     if let Message::Text(text) = msg {
        //         let uppercased = text.to_uppercase();
        //         if socket.send(Message::Text(uppercased.into())).await.is_err() {
        //             return;
        //         }
        //     }
        // }
    }))
}
