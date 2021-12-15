//! Example: Suspense
//! -----------------
//! This example demonstrates how the use_suspense hook can be used to load and render asynchronous data.  Suspense enables
//! components to wait on futures to complete before rendering the result into VNodes. These VNodes are immediately
//! available in a suspended" fashion and will automatically propagate to the UI when the future completes.
//!
//! Currently, suspense futures are non-restartable. In the future, we'll provide more granular control of how to start,
//! stop, and reset these futures.

use dioxus::prelude::*;
#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}
const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

pub static Example: Component<()> = |cx| {
    let doggo = use_suspense(
        cx,
        || surf::get(ENDPOINT).recv_json::<DogApi>(),
        |cx, res| match res {
            Ok(res) => rsx!(
                cx,
                img {
                    src: "{res.message}"
                }
            ),
            Err(_) => rsx!(cx, div { "No doggos for you :(" }),
        },
    );

    cx.render(rsx!(
        div {
            h1 {"Waiting for a doggo..."}
            {doggo}
        }
    ))
};
