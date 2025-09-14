#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_fullstack::Streaming;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut chat_response = use_signal(String::default);

    let mut send_request = use_action(move |e: FormEvent| async move {
        let value = e.values()["message-input"]
            .first()
            .cloned()
            .context("Missing message input")?;

        let mut response = get_chat_response(value).await?;

        while let Some(Ok(chunk)) = response.next().await {
            chat_response.write().push_str(&chunk);
        }

        dioxus::Ok(())
    });

    rsx! {
        form {
            onsubmit: move |e| send_request.dispatch(e),
            input { name: "message-input", placeholder: "Talk to your AI" }
            button { "Send" }
        }
    }
}

#[post("/api/chat")]
async fn get_chat_response(user_message: String) -> Result<Streaming<String>> {
    todo!()
}
