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
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        h1 { "Hot patch serverfns!" }
        button {
            onclick: move |_| async move {
                text.set(say_hi().await.unwrap());
            },
            "Say hi!"
        }
        "Server said: {text}"
    }
}

#[server]
async fn say_hi() -> Result<String, ServerFnError> {
    Ok("DUAL PATCHING ACHIEVED!".to_string())
}
