#![allow(non_snake_case, unused)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[derive(serde::Deserialize, Debug)]
struct ApiResponse {
    #[serde(rename = "message")]
    image_url: String,
}

#[rustfmt::skip]
fn App(cx: Scope) -> Element {
// ANCHOR: use_future
let future = use_future(cx, (), |_| async move {
    reqwest::get("https://dog.ceo/api/breeds/image/random")
        .await
        .unwrap()
        .json::<ApiResponse>()
        .await
});
// ANCHOR_END: use_future

// ANCHOR: render
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
// ANCHOR_END: render
}

#[rustfmt::skip]
#[inline_props]
fn RandomDog(cx: Scope, breed: String) -> Element {
// ANCHOR: dependency
let future = use_future(cx, (breed,), |(breed,)| async move {
    reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
        .await
        .unwrap()
        .json::<ApiResponse>()
        .await
});
// ANCHOR_END: dependency

    cx.render(rsx!(()))
}
