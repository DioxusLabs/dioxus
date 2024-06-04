use dioxus_lib::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::{cell::Cell, future::Future, rc::Rc};

use crate::html_storage::use_serialize_context;

/// A future that resolves to a value.
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_server_future<T, F, D: Dependency>(
    dependencies: D,
    mut future: impl FnMut(D::Out) -> F + 'static,
) -> Result<Resource<T>, RenderError>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    let first_run = use_hook(|| Rc::new(Cell::new(true)));
    let mut last_state = use_signal(|| {
        first_run.set(false);
        dependencies.out()
    });
    if !first_run.get() && dependencies.changed(&*last_state.peek()) {
        last_state.set(dependencies.out());
    }

    let cb = use_callback(move || future(last_state()));
    let mut first_run = use_hook(|| CopyValue::new(true));

    #[cfg(feature = "server")]
    let serialize_context = use_serialize_context();

    let resource = use_resource(move || {
        #[cfg(feature = "server")]
        let serialize_context = serialize_context.clone();

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
                    return o;
                }
            }

            // Otherwise just run the future itself
            let out = user_fut.await;

            // If this is the first run and we are on the server, cache the data
            #[cfg(feature = "server")]
            if currently_in_first_run {
                serialize_context.push(&out);
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
