//! Example: Suspense
//! -----------------
//! This example shows how the "use_fetch" hook is built on top of Dioxus' "suspense" API. Suspense enables components
//! to wait on futures to complete before rendering the result into VNodes. These VNodes are immediately available in a
//! "suspended" fashion and will automatically propogate to the UI when the future completes.
//!
//! Note that if your component updates or receives new props while it is awating the result, the future will be dropped
//! and all work will stop. In this example, we store the future in a hook so we can always resume it.

use dioxus::prelude::*;
fn main() {}
#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}
const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

pub static App: FC<()> = |cx| {
    let doggo = use_future_effect(&cx, move || async move {
        match surf::get(ENDPOINT).recv_json::<DogApi>().await {
            Ok(res) => rsx!(in cx, img { src: "{res.message}" }),
            Err(_) => rsx!(in cx, div { "No doggos for you :(" }),
        }
    });

    cx.render(rsx!(
        div {
            h1 {"Waiting for a doggo..."}
            {doggo}
        }
    ))
};

use dioxus_core::virtual_dom::SuspendedContext;
use futures::Future;
use futures::FutureExt;
use std::pin::Pin;
fn use_fetch<'a, T: serde::de::DeserializeOwned + 'static>(
    cx: &impl Scoped<'a>,
    url: &str,
    g: impl FnOnce(SuspendedContext, surf::Result<T>) -> VNode<'a> + 'a,
) -> VNode<'a> {
    // essentially we're doing a "use_effect" but with no dependent props or post-render shenanigans
    let fetch_promise = cx.use_hook(
        move || surf::get(url).recv_json::<T>().boxed_local(),
        // just pass the future through
        |p| p,
        |_| (),
    );
    cx.suspend(fetch_promise, g)
}

/// Spawns the future only when the inputs change
fn use_future_effect<'a, 'b, F: Future<Output = VNode<'b>>>(
    cx: &impl Scoped<'a>,
    g: impl FnOnce() -> F + 'a,
) -> VNode<'a> {
    todo!()
}
