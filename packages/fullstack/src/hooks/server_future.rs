use dioxus_lib::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::cell::Cell;
use std::cell::Ref;
use std::cell::RefCell;
use std::fmt::Debug;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

/// A future that resolves to a value.
///
///
///
/// ```rust
/// fn User(id: String) -> Element {
///     let data = use_sever_future(move || fetch_user(id)).suspend()?;
///
///
/// }
///
/// ```
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_server_future<T, F>(_future: impl Fn() -> F) -> UseServerFuture<T>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    let value: Signal<Option<T>> = use_signal(|| {
        // Doesn't this need to be keyed by something?
        // We should try and link these IDs across the server and client
        // Just the file/line/col span should be fine (or byte index)
        #[cfg(feature = "ssr")]
        return crate::html_storage::deserialize::take_server_data::<T>();

        #[cfg(not(feature = "ssr"))]
        return None;
    });

    // Run the callback regardless, giving us the future without actually polling it
    // This is where use_server_future gets its reactivity from
    // If the client is using signals to drive the future itself, (say, via args to the server_fn), then we need to know
    // what signals are being used
    use_future(move || async move {
        // watch the reactive context
        // if it changes, restart the future
        //
        // if let Err(err) = crate::prelude::server_context().push_html_data(&data) {
        //     tracing::error!("Failed to push HTML data: {}", err);
        // };
    });

    // if there's no value ready, mark this component as suspended and return early
    if value.peek().is_none() {
        suspend();
    }

    todo!()
}

pub struct UseServerFuture<T: 'static> {
    value: Signal<Option<Signal<T>>>,
}

// impl<T> UseServerFuture<T> {
//     /// Restart the future with new dependencies.
//     ///
//     /// Will not cancel the previous future, but will ignore any values that it
//     /// generates.
//     pub fn restart(&self) {
//         self.needs_regen.set(true);
//         (self.update)();
//     }

//     /// Forcefully cancel a future
//     pub fn cancel(&self) {
//         if let Some(task) = self.task.take() {
//             remove_future(task);
//         }
//     }

//     /// Return any value, even old values if the future has not yet resolved.
//     ///
//     /// If the future has never completed, the returned value will be `None`.
//     pub fn value(&self) -> Ref<'_, T> {
//         Ref::map(self.value.borrow(), |v| v.as_deref().unwrap())
//     }

//     /// Get the ID of the future in Dioxus' internal scheduler
//     pub fn task(&self) -> Option<Task> {
//         self.task.get()
//     }

//     /// Get the current state of the future.
//     pub fn reloading(&self) -> bool {
//         self.task.get().is_some()
//     }
// }
