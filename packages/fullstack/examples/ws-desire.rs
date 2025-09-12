use dioxus::prelude::*;
use dioxus_fullstack::Websocket;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut ws = use_websocket(|| uppercase_ws("John Doe".into(), 30));
    let mut message = use_signal(|| "Send a message!".to_string());

    if ws.connecting() {
        return rsx! { "Connecting..." };
    }

    rsx! {
        input {
            oninput: move |e| async move {
                _ = ws.send(()).await;
            },
            placeholder: "Type a message",
        }
    }
}

#[get("/api/uppercase_ws?name&age")]
async fn uppercase_ws(name: String, age: i32) -> anyhow::Result<Websocket> {
    use axum::extract::ws::Message;

    Ok(Websocket::raw(|mut socket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            if let Message::Text(text) = msg {
                let response = format!("Hello {}, you are {} years old!", name, age);
                socket.send(Message::Text(response.into())).await.unwrap();
            }
        }
    }))
}
