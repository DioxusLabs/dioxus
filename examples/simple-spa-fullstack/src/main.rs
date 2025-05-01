//! Simple single-page-app setup.
//!
//!  Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::{logger::tracing, prelude::*};

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
            EvalIt { color: "red" }
            EvalIt { color: "orange" }
            EvalIt { color: "yellow" }
            EvalIt { color: "green" }
            EvalIt { color: "blue" }
            EvalIt { color: "indigo" }
            EvalIt { color: "violet" }
            EvalIt { color: "pink" }
            EvalIt { color: "cyan" }
            EvalIt { color: "lime" }
            EvalIt { color: "teal" }
            EvalIt { color: "brown" }
            EvalIt { color: "gray" }
            EvalIt { color: "white" }
            EvalIt { color: "black" }
            EvalIt { color: "magenta" }
            EvalIt { color: "maroon" }
            EvalIt { color: "navy" }
            EvalIt { color: "olive" }
            EvalIt { color: "silver" }
        }
        button {
            onclick: move |_| async move {
                if let Ok(data) = get_server_data().await {
                    text.set(data.clone());
                    tracing::debug!("Sending: {}", data);
                    let res = post_server_data(data).await;
                    tracing::debug!("res: {:?}", res);
                }

            },
            "Run a server function!"
        }
        button {
            onclick: move |_| async move {
                // if let Ok(data) = get_curr_time().await {
                //     text.set(data.clone());
                tracing::debug!("Sending: {}", t.to_string());
                let res = post_server_data(t.to_string()).await;
                tracing::debug!("res: {:?}", res);
                if let Ok(data) = res {
                    text.set(data);
                }
                // }
            },
            "Run a server function with data!"
        }
        // button {
        //     onclick: move |_| {
        //         let items = get_select_data_list("hello".to_string());
        //         tracing::debug!("items: {:?}", items);
        //     }
        // }
        "Server said: {text}"
        "Server said: {text}"

    }
}

#[server(endpoint = "/post_server_data")]
async fn post_server_data(data: String) -> Result<String, ServerFnError> {
    tracing::debug!("Server data: {data}");
    Ok(format!("Server received: {data}"))
}

#[server(endpoint = "/get_server_data")]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

#[server(endpoint = "/get_curr_time")]
async fn get_curr_time() -> Result<String, ServerFnError> {
    Ok(format!(
        "current time: -> {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ))
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

use wasm_bindgen::prelude::*;
// web-sys does not expose the keys api for select data, so we need to manually bind to it
#[wasm_bindgen(inline_js = r#"
export function get_select_data_list(select) {
    let values = [select];

    return values;
}
"#)]
extern "C" {
    fn get_select_data_list(item: String) -> Vec<String>;
}
