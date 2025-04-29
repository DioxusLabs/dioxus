//! Simple single-page-app setup.
//!
//!  Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut t = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        h1 { "Set your favorite color" }
        button { onclick: move |_| t += 1, "Click me: {t}" }
        div {
            EvalIt { color: "white" }
            EvalIt { color: "red" }
            EvalIt { color: "yellow" }
        }
        button {
            onclick: move |_| async move {
                if let Ok(data) = get_server_data().await {
                    text.set(data.clone());
                    post_server_data(data).await.unwrap();
                }

            },
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
            "Run a server function!"
        }
        "Server said: {text}"

    }
}

#[server]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    println!("Server received: {}", data);

    Ok(())
}

#[server]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

#[component]
fn EvalIt(color: String) -> Element {
    rsx! {
        div {
            button {
                onclick: move |_| {
                    _ = dioxus::document::eval(&format!("window.document.body.style.backgroundColor = '{color}';"));
                },
                "eval -> {color}"
            }
        }
    }
}
