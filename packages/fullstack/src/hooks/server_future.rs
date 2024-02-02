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
    let mut gen = use_hook(|| CopyValue::new(0));

    let resource = use_resource(move || {
        async move {
            // this is going to subscribe this resource to any reactivity given to use in the callback
            // We're doing this regardless so inputs get tracked, even if we drop the future before polling it
            let user_fut = cb.call();

            // If this is the first run, the data might be cached
            if gen() == 0 {
                #[cfg(not(feature = "web"))]
                if let Some(o) = crate::html_storage::deserialize::take_server_data::<T>() {
                    gen.set(1);
                    return o;
                }
            }

            // Otherwise just run the future itself
            let out = user_fut.await;

            // and push the gen forward
            gen.set(1);

            out
        }
    });

    // On the first run, force this task to be polled right away in case its value is ready
    use_hook(|| {
        let _ = resource.task().unwrap().poll_now();
    });

    // Suspend if the value isn't ready
    match resource.state() {
        UseResourceState::Pending => {
            suspend();
            None
        }
        UseResourceState::Regenerating => {
            suspend();
            Some(resource)
        }
        UseResourceState::Ready => Some(resource),
    }
}
