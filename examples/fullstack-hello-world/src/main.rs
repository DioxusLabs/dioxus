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
            "Request from the  server"
        }
        div {
            "server says: {text}"
        }
    }
}

#[server]
async fn do_server_action() -> Result<String, ServerFnError> {
    Ok("hello from the server - hotpatched!!".to_string())
}
