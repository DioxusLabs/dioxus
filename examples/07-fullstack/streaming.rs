use dioxus::{
    fullstack::{JsonEncoding, Streaming, TextStream},
    prelude::*,
};

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut text_responses = use_signal(String::new);
    let mut json_responses = use_signal(Vec::new);

    let mut cancellable_stream = use_action(move || async move {
        text_responses.write().clear();
        let mut stream = text_stream(Some(100)).await?;
        while let Some(Ok(text)) = stream.next().await {
            text_responses.write().push_str(&text);
            text_responses.write().push('\n');
        }

        dioxus::Ok(())
    });

    rsx! {
        div {
            button {
                onclick: move |_| async move {
                    text_responses.write().clear();
                    let mut stream = text_stream(Some(100)).await?;
                    while let Some(Ok(text)) = stream.next().await {
                        text_responses.write().push_str(&text);
                        text_responses.write().push('\n');
                    }
                    Ok(())
                },
                "Start text stream"
            }
            button {
                onclick: move |_| {
                    cancellable_stream.call();
                },
                "Start cancellable text stream"
            }
            button {
                onclick: move |_| {
                    cancellable_stream.cancel();
                },
                "Stop cancellable text stream"
            }

            pre { "{text_responses}" }
        }
        div {
            button {
                onclick: move |_| async move {
                    json_responses.write().clear();
                    let mut stream = json_stream().await?;
                    while let Some(Ok(dog)) = stream.next().await {
                        json_responses.write().push(dog);
                    }
                    Ok(())
                },
                "Start JSON stream"
            }
            div {
                for dog in json_responses.read().iter() {
                    pre { "{dog:?}" }
                }
            }
        }
    }
}

#[get("/api/test_stream?start")]
async fn text_stream(start: Option<i32>) -> Result<TextStream> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut count = start.unwrap_or(0);
        loop {
            if tx
                .unbounded_send(format!("Hello, world! {}", count))
                .is_err()
            {
                // If the channel is closed, stop sending chunks
                break;
            }
            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Streaming::new(rx))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Dog {
    name: String,
    age: u8,
}

#[get("/api/json_stream")]
async fn json_stream() -> Result<Streaming<Dog, JsonEncoding>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut count = 0;
        loop {
            let dog = Dog {
                name: format!("Dog {}", count),
                age: (count % 10) as u8,
            };
            if tx.unbounded_send(dog).is_err() {
                // If the channel is closed, stop sending chunks
                break;
            }
            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Streaming::new(rx))
}
