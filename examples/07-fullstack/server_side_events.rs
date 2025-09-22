#![allow(non_snake_case)]

use anyhow::{Context, Result};
use dioxus::prelude::*;
use dioxus_fullstack::{ServerEvents, Streaming};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut events = use_signal(Vec::new);
    use_future(move || async move {
        let mut stream = listen_for_changes().await?;

        while let Some(Ok(event)) = stream.next().await {
            events.write().push(event);
        }

        anyhow::Ok(())
    });

    rsx! {
        h1 { "Events from server: " }
        ul {
            for msg in events.iter() {
                li { "{msg}" }
            }
        }
    }
}

#[get("/api/sse")]
async fn listen_for_changes() -> Result<ServerEvents<String>> {
    use std::time::Duration;

    Ok(ServerEvents::new(|mut tx| async move {
        loop {
            // Poll some data source here, subscribe to changes?
            tokio::time::sleep(Duration::from_secs(1)).await;

            if tx.send("hello world".to_string()).await.is_err() {
                // client disconnected, do some cleanup
                break;
            }
        }
    }))
}
