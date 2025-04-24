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
                text.set(do_server_action1().await.unwrap());
            },
            "Request from the server 1"
        }
        p { "server says: {text}" }
        Child { idx: 1 }
        Child { idx: 2 }
        Child { idx: 3 }
        Child { idx: 4 }
        Child { idx: 5 }
        Child { idx: 6 }
        Child { idx: 7 }
        Child { idx: 8 }
        Child { idx: 9 }
        Child { idx: 10 }
        Child { idx: 11 }
        Child { idx: 12 }
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
