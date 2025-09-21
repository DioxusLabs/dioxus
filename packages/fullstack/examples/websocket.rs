use dioxus::prelude::*;
use dioxus_fullstack::{use_websocket, WebSocketOptions, Websocket};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let tx = use_loader(|| uppercase_ws("John Doe".into(), 30, WebSocketOptions::new()))?;

    rsx! {
        input {
            oninput: move |e| async move {
                // tx.send(e.value());
            },
            placeholder: "Type a message",
        }
    }
}

#[get("/api/uppercase_ws?name&age")]
async fn uppercase_ws(name: String, age: i32, options: WebSocketOptions) -> Result<Websocket> {
    use axum::extract::ws::Message;

    Ok(options.on_upgrade(move |mut socket| async move {
        let greeting = format!("Hello, {}! You are {} years old.", name, age);
        if socket.send(Message::Text(greeting.into())).await.is_err() {
            return;
        }

        while let Some(Ok(msg)) = socket.recv().await {
            if let Message::Text(text) = msg {
                let uppercased = text.to_uppercase();
                if socket.send(Message::Text(uppercased.into())).await.is_err() {
                    return;
                }
            }
        }
    }))
}
