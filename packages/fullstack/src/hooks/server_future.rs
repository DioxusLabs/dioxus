use dioxus_lib::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// A future that resolves to a value.
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_server_future<T, F>(
    _future: impl FnMut() -> F + 'static,
) -> Result<Resource<T>, RenderError>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    let cb = use_callback(_future);
    let mut first_run = use_hook(|| CopyValue::new(true));

    let resource = use_resource(move || {
        async move {
            let user_fut = cb.call();

            let currently_in_first_run = first_run.cloned();

            // If this is the first run and we are on the web client, the data might be cached
            if currently_in_first_run {
                tracing::info!("First run of use_server_future");
                // This is no longer the first run
                first_run.set(false);

                #[cfg(feature = "web")]
                if let Some(o) = crate::html_storage::deserialize::take_server_data::<T>() {
                    // this is going to subscribe this resource to any reactivity given to use in the callback
                    // We're doing this regardless so inputs get tracked, even if we drop the future before polling it
                    kick_future(user_fut);

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

#[cfg(feature = "web")]
#[inline]
fn kick_future<F, T>(user_fut: F)
where
    F: Future<Output = T> + 'static,
{
    // Kick the future to subscribe its dependencies
    use futures_util::future::FutureExt;
    let waker = futures_util::task::noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    futures_util::pin_mut!(user_fut);

    let _ = user_fut.poll_unpin(&mut cx);
}
