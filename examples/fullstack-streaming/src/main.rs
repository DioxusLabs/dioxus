use dioxus::prelude::*;
use futures::StreamExt;
use server_fn::codec::{StreamingText, TextStream};

fn app() -> Element {
    let mut response = use_signal(String::new);

    rsx! {
        button {
            onclick: move |_| async move {
                response.write().clear();
                if let Ok(stream) = test_stream().await {
                    response.write().push_str("Stream started\n");
                    let mut stream = stream.into_inner();
                    while let Some(Ok(text)) = stream.next().await {
                        response.write().push_str(&text);
                    }
                }
            },
            "Start stream"
        }
        "{response}"
    }
}

#[server(output = StreamingText)]
pub async fn test_stream() -> Result<TextStream, ServerFnError> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let _ = tx.unbounded_send(Ok("Hello, world!".to_string()));
        }
    });

    Ok(TextStream::new(rx))
}

fn main() {
    launch(app)
}
