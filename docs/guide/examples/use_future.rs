#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

#[derive(serde::Deserialize, Debug)]
struct ApiResponse {
    #[serde(rename = "message")]
    image_url: String,
}

fn App(cx: Scope) -> Element {
    let future = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await
            .unwrap()
            .json::<ApiResponse>()
            .await
    });

    cx.render(match future.value() {
        Some(Ok(response)) => rsx! {
            button {
                onclick: move |_| future.restart(),
                "Click to fetch another doggo"
            }
            div {
                img {
                    max_width: "500px",
                    max_height: "500px",
                    src: "{response.image_url}",
                }
            }
        },
        Some(Err(_)) => rsx! { div { "Loading dogs failed" } },
        None => rsx! { div { "Loading dogs..." } },
    })
}
