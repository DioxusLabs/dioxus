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
    let doggo = use_fetch(&cx, ENDPOINT, |cx, res: surf::Result<DogApi>| match res {
        Ok(res) => rsx!(in cx, img { src: "{res.message}"}),
        Err(_) => rsx!(in cx, p { "Failed to load doggo :("}),
    });

    cx.render(rsx!(
        div {
            h1 {"Waiting for a doggo..."}
            {doggo}
        }
    ))
};

fn use_fetch<'a, T: serde::de::DeserializeOwned + 'static>(
    cx: &impl Scoped<'a>,
    url: &str,
    g: impl FnOnce(dioxus_core::virtual_dom::SuspendedContext, surf::Result<T>) -> VNode<'a> + 'a,
) -> VNode<'a> {
    use futures::Future;
    use futures::FutureExt;
    use std::pin::Pin;

    // essentially we're doing a "use_effect" but with no dependent props
    let doggo_promise: &'a mut Pin<Box<dyn Future<Output = surf::Result<T>> + Send + 'static>> = cx
        .use_hook(
            move || surf::get(url).recv_json::<T>().boxed(),
            // just pass the future through
            |p| p,
            |_| (),
        );
    cx.suspend(doggo_promise, g)
}
