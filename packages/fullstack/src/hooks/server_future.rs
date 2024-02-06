use dioxus_lib::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// A future that resolves to a value.
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_server_future<T, F>(_future: impl Fn() -> F + 'static) -> Option<Resource<T>>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    let mut cb = use_callback(_future);
    let mut first_run = use_hook(|| CopyValue::new(true));

    let resource = use_resource(move || {
        async move {
            // this is going to subscribe this resource to any reactivity given to use in the callback
            // We're doing this regardless so inputs get tracked, even if we drop the future before polling it
            let user_fut = cb.call();

            let currently_in_first_run = first_run.cloned();

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

            // If this is the first run and we are on the server, cache the data
            #[cfg(feature = "server")]
            if currently_in_first_run {
                let _ = crate::server_context::server_context().push_html_data(&out);
            }

            #[allow(clippy::let_and_return)]
            out
        }
    });

    // On the first run, force this task to be polled right away in case its value is ready
    use_hook(|| {
        let _ = resource.task().poll_now();
    });

    // Suspend if the value isn't ready
    match resource.state().cloned() {
        UseResourceState::Pending => {
            suspend();
            None
        }
        _ => Some(resource),
    }
}
