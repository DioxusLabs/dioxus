use dioxus::prelude::*;
use dioxus_fullstack::ServerEvents;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut events = use_signal(Vec::new);

    use_future(move || async move {
        // Call the SSE endpoint to get a stream of events
        let mut stream = listen_for_changes().await?;

        // And then poll it for new events, adding them to our signal
        while let Some(Ok(event)) = stream.next().await {
            events.write().push(event);
        }

        anyhow::Ok(())
    });

    rsx! {
        h1 { "Events from server: " }
        for msg in events.read().iter().rev() {
            pre { "{msg:?}" }
        }
    }
}

/// We can send anything that's serializable as a server event - strings, numbers, structs, enums, etc.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
enum MyServerEvent {
    Yay { message: String },
    Nay { error: String },
}

/// Our SSE endpoint, when called, will return the ServerEvents handle which streams events to the client.
/// On the client, we can interact with this stream object to get new events as they arrive.
#[get("/api/sse")]
async fn listen_for_changes() -> Result<ServerEvents<MyServerEvent>> {
    use std::time::Duration;

    Ok(ServerEvents::new(|mut tx| async move {
        let mut count = 1;

        loop {
            // Create our serializable message
            let msg = if count % 5 == 0 {
                MyServerEvent::Nay {
                    error: "An error occurred".into(),
                }
            } else {
                MyServerEvent::Yay {
                    message: format!("Hello number {count}"),
                }
            };

            // Send the message to the client. If it errors, the client has disconnected
            if tx.send(msg).await.is_err() {
                // client disconnected, do some cleanup
                break;
            }

            count += 1;

            // Poll some data source here, subscribe to changes, maybe call an LLM?
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }))
}
