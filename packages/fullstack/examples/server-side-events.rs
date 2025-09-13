#![allow(non_snake_case)]

use anyhow::{Context, Result};
use axum::response::sse::{Event, KeepAlive, Sse};
use dioxus::prelude::*;
use dioxus_fullstack::{ServerSentEvents, Streaming};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut events = use_signal(Vec::new);

    use_future(move || async move {
        let mut stream = listen_for_changes()
            .await
            .context("failed to listen for changes")?;

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
async fn listen_for_changes() -> Result<ServerSentEvents<String>> {
    todo!()
}
