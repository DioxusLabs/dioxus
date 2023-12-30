//! Run with:
//!
//! ```sh
//! dx build --features web --release
//! cargo run --features ssr --release
//! ```

#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_fullstack::{
    launch::{self, LaunchBuilder},
    prelude::*,
};
use serde::{Deserialize, Serialize};

#[component]
fn app(cx: Scope) -> Element {
    let text =
        use_server_future(cx, (), |()| async move { get_server_data().await.unwrap() })?.value();

    #[cfg(not(feature = "ssr"))]
    panic!("This component will only ever be rendered on the server!");

    cx.render(rsx! { Child { state: text.clone() } })
}

#[component(island)]
fn Child(cx: Scope, state: String) -> Element {
    cx.render(rsx! {"State: {state}"})
}

#[server]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}

fn main() {
    LaunchBuilder::new(app).launch()
}
