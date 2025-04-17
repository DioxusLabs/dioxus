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
        button {
            onclick: move |_| async move {
                text.set(say_hi().await.unwrap());
                text.set(say_hi().await.unwrap());
            },
            "Say hi!"
        }
        "Server said: {text}"
        Child2 { i: 123 }
    }
}

#[component]
fn Child2(i: i32) -> Element {
    rsx! {
        div { "Hello from the child component!" }
    }
}

#[server]
async fn say_hi() -> Result<String, ServerFnError> {
    Ok("DUAL asdasd ACHIEVED?asdasdads????!".to_string())
}

#[server]
async fn say_bye() -> Result<String, ServerFnError> {
    Ok("goodbye!".to_string())
}

#[server]
async fn say_bye2() -> Result<String, ServerFnError> {
    Ok("goodbye1!".to_string())
}

#[server]
async fn say_bye3() -> Result<String, ServerFnError> {
    Ok("goodbye2!".to_string())
}
