//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut text = use_signal(|| "make a request!".to_string());

    rsx! {
        h1 { "Hot patch serverfns!" }
        button {
            onclick: move |_| async move {
                text.set(do_server_action().await.unwrap());
            },
            "Request from the server 1"
        }
        button {
            onclick: move |_| async move {
                text.set(do_server_action2().await.unwrap());
            },
            "Request from the server 2"
        }
        p {
            "server says: {text}"
        }
    }
}

#[server]
async fn do_server_action() -> Result<String, ServerFnError> {
    Ok("hello from the server - hotpatched!! server fn".to_string())
}

#[server]
async fn do_server_action2() -> Result<String, ServerFnError> {
    Ok("server action 2".to_string())
}
