//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

static CSS: Asset = asset!("/assets/hello.css");

fn app() -> Element {
    let mut text = use_signal(|| "make a request!".to_string());

    rsx! {
        h1 { "Hot patch serverfns!" }
        link { rel: "stylesheet", href: CSS }
        button {
            onclick: move |_| async move {
                text.set(do_server_action1().await.unwrap());
            },
            "Request from the server 1"
        }
        button {
            onclick: move |_| async move {
                text.set(do_server_action2().await.unwrap());
            },
            "Request from the server 2"
        }
        p { "server says: {text}" }
        Child { idx: 1 }
    }
}

#[component]
fn Child(idx: i32) -> Element {
    rsx! {
        div { "hi -> {idx}" }
    }
}

#[server]
async fn do_server_action1() -> Result<String, ServerFnError> {
    Ok("hello from the 123123123server!!".to_string())
}

#[server]
async fn do_server_action2() -> Result<String, ServerFnError> {
    Ok("server action 3123123".to_string())
}
