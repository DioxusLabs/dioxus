use dioxus_lib::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// Runs a future with a manual list of dependencies and returns a resource with the result if the future is finished or a suspended error if it is still running.
///
///
/// On the server, this will wait until the future is resolved before continuing to render. When the future is resolved, the result will be serialized into the page and hydrated on the client without rerunning the future.
///
///
/// <div class="warning">
///
/// Unlike [`use_resource`] dependencies are only tracked inside the function that spawns the async block, not the async block itself.
///
/// ```rust
/// // ❌ The future inside of use_server_future is not reactive
/// let id = use_signal(|| 0);
/// use_server_future(move || {
///     async move {
///          // But the future is not reactive which means that the future will not subscribe to any reads here
///          println!("{id}");
///     }
/// });
/// // ✅ The closure that creates the future for use_server_future is reactive
/// let id = use_signal(|| 0);
/// use_server_future(move || {
///     // The closure itself is reactive which means the future will subscribe to any signals you read here
///     let cloned_id = id();
///     async move {
///          // But the future is not reactive which means that the future will not subscribe to any reads here
///          println!("{cloned_id}");
///     }
/// });
/// ```
///
/// </div>
///
/// # Example
///
/// ```rust
/// # async fn fetch_article(id: u32) -> String { todo!() }
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     let mut article_id = use_signal(|| 0);
///     // `use_server_future` will spawn a task that runs on the server and serializes the result to send to the client.
///     // The future will rerun any time the
///     // Since we bubble up the suspense with `?`, the server will wait for the future to resolve before rendering
///     let article = use_server_future(move || fetch_article(article_id()))?;
///
///     rsx! {
///         "{article().unwrap()}"
///     }
/// }
/// ```
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_server_future<T, F>(
    mut future: impl FnMut() -> F + 'static,
) -> Result<Resource<T>, RenderError>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    #[cfg(feature = "server")]
    let serialize_context = crate::html_storage::use_serialize_context();
    // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
    #[cfg(feature = "server")]
    let server_storage_entry = use_hook(|| serialize_context.create_entry());

    let mut first_run = use_hook(|| CopyValue::new(true));

    let resource = use_resource(move || {
        #[cfg(feature = "server")]
        let serialize_context = serialize_context.clone();
        let user_fut = future();

        async move {
            let currently_in_first_run = first_run();

            // If this is the first run and we are on the web client, the data might be cached
            if currently_in_first_run {
                tracing::info!("First run of use_server_future");
                // This is no longer the first run
                first_run.set(false);

                #[cfg(feature = "web")]
                if let Some(o) = crate::html_storage::deserialize::take_server_data::<T>() {
                    return o;
                }
            }

            // Otherwise just run the future itself
            let out = user_fut.await;

            // If this is the first run and we are on the server, cache the data in the slot we reserved for it
            #[cfg(feature = "server")]
            if currently_in_first_run {
                serialize_context.insert(server_storage_entry, &out);
            }

            #[allow(clippy::let_and_return)]
            out
        }
    });

    // On the first run, force this task to be polled right away in case its value is ready
    use_hook(|| {
        let _ = resource.task().map(|task| task.poll_now());
    });

    // Suspend if the value isn't ready
    match resource.state().cloned() {
        UseResourceState::Pending => {
            if let Some(task) = resource.task() {
                return Err(suspend(task).unwrap_err());
            }
            Ok(resource)
        }
        _ => Ok(resource),
    }
}
