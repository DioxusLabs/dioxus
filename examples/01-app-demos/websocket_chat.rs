//! A websocket chat demo using Dioxus' built-in websocket support.
//!
//! We setup an endpoint at `/api/chat` that accepts a `name` and `user_id` query parameter.
//! Each client connects to that endpoint, and we use a `tokio::broadcast` channel
//! to send messages to all connected clients.
//!
//! In practice, you'd use a distributed messaging system (Redis PubSub / Kafka / etc) to coordinate
//! between multiple server instances and an additional database to persist chat history.

use dioxus::fullstack::{WebSocketOptions, Websocket, use_websocket};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // store the user's current input
    let mut input = use_signal(|| "".to_string());

    // Select a unique id for the user, and then use that entropy to pick a random name
    let user_id = use_signal(uuid::Uuid::new_v4);
    let user_name = use_signal(|| {
        match user_id.read().as_bytes()[0] % 7 {
            0 => "Alice",
            1 => "Bob",
            2 => "Eve",
            3 => "Mallory",
            4 => "Trent",
            5 => "Peggy",
            6 => "Victor",
            _ => "Charlie",
        }
        .to_string()
    });

    // Store the messages we've received from the server
    let mut message_list = use_signal(Vec::<ChatMessage>::new);

    // Connect to the websocket endpoint
    let mut socket =
        use_websocket(move || uppercase_ws(user_name(), user_id(), Default::default()));

    use_future(move || async move {
        while let Ok(msg) = socket.recv().await {
            match msg {
                ServerEvent::ReceiveMessage(message) => message_list.push(message),
                ServerEvent::Connected { messages } => message_list.set(messages),
            }
        }
    });

    rsx! {
        h1 { "WebSocket Chat" }
        p { "Connection status: {socket.status():?} as {user_name}" }
        input {
            placeholder: "Type a message",
            value: "{input}",
            oninput: move |e| async move { input.set(e.value()) },
            onkeydown: move |e| async move {
                if e.key() == Key::Enter {
                    _ = socket.send(ClientEvent::SendMessage(input.read().clone())).await;
                    input.set("".to_string());
                }
            }
        }

        div {
            for message in message_list.read().iter().rev() {
                pre { "{message.name}: {message.message}" }
            }
        }
    }
}

/// The events that the client can send to the server
#[derive(Serialize, Deserialize, Debug)]
enum ClientEvent {
    SendMessage(String),
}

/// The events that the server can send to the client
#[derive(Serialize, Deserialize, Debug)]
enum ServerEvent {
    Connected { messages: Vec<ChatMessage> },
    ReceiveMessage(ChatMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChatMessage {
    user_id: Uuid,
    name: String,
    message: String,
}

#[get("/api/chat?name&user_id")]
async fn uppercase_ws(
    name: String,
    user_id: Uuid,
    options: WebSocketOptions,
) -> Result<Websocket<ClientEvent, ServerEvent>> {
    use std::sync::LazyLock;
    use tokio::sync::{
        Mutex,
        broadcast::{self, Sender},
    };

    // Every chat app needs a chat room! For this demo, we just use a tokio broadcast channel and a mutex-protected
    // list of messages to store chat history.
    //
    // We place these types in the body of this serverfn since they're not used on the client, only the server.
    static MESSAGES: LazyLock<Mutex<Vec<ChatMessage>>> = LazyLock::new(|| Mutex::new(Vec::new()));
    static BROADCAST: LazyLock<Sender<ChatMessage>> = LazyLock::new(|| broadcast::channel(100).0);

    Ok(options.on_upgrade(move |mut socket| async move {
        // Send back all the messages from the room to the new client
        let messages = MESSAGES.lock().await.clone();
        _ = socket.send(ServerEvent::Connected { messages }).await;

        // Subscriber to the broadcast channel
        let sender = BROADCAST.clone();
        let mut broadcast = sender.subscribe();

        // Announce that we've joined
        let _ = sender.send(ChatMessage {
            message: format!("{name} has connected."),
            user_id,
            name: "[CONSOLE]".to_string(),
        });

        // Loop poll the broadcast receiver and the websocket for new messages
        // If we receive a message from the broadcast channel, send it to the client
        // If we receive a message from the client, broadcast it to all other clients and save it to the message list
        loop {
            tokio::select! {
                Ok(msg) = broadcast.recv() => {
                    let _ = socket.send(ServerEvent::ReceiveMessage(msg)).await;
                }
                Ok(ClientEvent::SendMessage(message)) = socket.recv() => {
                    let chat_message = ChatMessage {
                        user_id,
                        name: name.clone(),
                        message,
                    };
                    let _ = sender.send(chat_message.clone());
                    MESSAGES.lock().await.push(chat_message.clone());
                },
                else => break,
            }
        }

        _ = sender.send(ChatMessage {
            name: "[CONSOLE]".to_string(),
            message: format!("{name} has disconnected."),
            user_id,
        });
    }))
}
