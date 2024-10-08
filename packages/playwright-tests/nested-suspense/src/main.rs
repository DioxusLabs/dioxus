// This test is used by playwright configured in the root of the repo
// Tests:
// - SEO without JS
// - Streaming hydration
// - Suspense
// - Server functions
//
// Without Javascript, content may not load into the right location, but it should still be somewhere in the html even if it is invisible

#![allow(non_snake_case)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    LaunchBuilder::fullstack().launch(app);
}

fn app() -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: move |_| rsx! {},
            LoadTitle {}
        }
        MessageWithLoader { id: 0 }
    }
}

#[component]
fn MessageWithLoader(id: usize) -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: move |_| rsx! {
                "Loading {id}..."
            },
            Message { id }
        }
    }
}

#[component]
fn LoadTitle() -> Element {
    let title = use_server_future(move || server_content(0))?()
        .unwrap()
        .unwrap();

    rsx! {
        document::Title { "{title.title}" }
    }
}

#[component]
fn Message(id: usize) -> Element {
    let message = use_server_future(move || server_content(id))?()
        .unwrap()
        .unwrap();

    rsx! {
        h2 {
            id: "title-{id}",
            "{message.title}"
        }
        p {
            id: "body-{id}",
            "{message.body}"
        }
        div {
            id: "children-{id}",
            padding: "10px",
            for child in message.children {
                MessageWithLoader { id: child }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Content {
    title: String,
    body: String,
    children: Vec<usize>,
}

#[server]
async fn server_content(id: usize) -> Result<Content, ServerFnError> {
    let content_tree = [
        Content {
            title: "The robot says hello world".to_string(),
            body: "The robot becomes sentient and says hello world".to_string(),
            children: vec![1, 2, 3],
        },
        Content {
            title: "The world says hello back".to_string(),
            body: "In a stunning turn of events, the world collectively unites and says hello back"
                .to_string(),
            children: vec![4],
        },
        Content {
            title: "Goodbye Robot".to_string(),
            body: "The robot says goodbye".to_string(),
            children: vec![],
        },
        Content {
            title: "Goodbye World".to_string(),
            body: "The world says goodbye".to_string(),
            children: vec![],
        },
        Content {
            title: "Hello World".to_string(),
            body: "The world says hello again".to_string(),
            children: vec![],
        },
    ];
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    Ok(content_tree[id].clone())
}
