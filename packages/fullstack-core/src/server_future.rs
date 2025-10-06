use dioxus_core::{suspend, use_hook, RenderError};
use dioxus_hooks::*;
use dioxus_signals::ReadableExt;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// Runs a future and returns a result with a resource if the future is finished or a suspended error if it is still running. The future may run on the server *during hydration*.
/// - When compiled as server, the closure's future is ran to completion and the resulting data is serialized on the server and sent to the client.
/// - When compiled as web client, the data is deserialized from the server if already available, otherwise runs on the client. Data is usually only available if this hook exists in a component during hydration.
/// - When otherwise compiled, the closure is run directly with no serialization.
///
/// On the server, this will wait until the future is resolved before continuing to render. Thus, this blocks other subsequent server hooks. The result is cached.
///
///
/// <div class="warning">
///
/// Unlike [`use_resource`] dependencies are only tracked inside the function that spawns the async block, not the async block itself.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
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
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # async fn fetch_article(id: u32) -> String { unimplemented!() }
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
#[track_caller]
pub fn use_server_future<T, F>(
    mut future: impl FnMut() -> F + 'static,
) -> Result<Resource<T>, RenderError>
where
    T: Serialize + DeserializeOwned + 'static,
    F: Future<Output = T> + 'static,
{
    let serialize_context = use_hook(crate::serialize_context);

    // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
    #[allow(unused)]
    let storage_entry: crate::SerializeContextEntry<T> =
        use_hook(|| serialize_context.create_entry());

    #[cfg(feature = "server")]
    let caller = std::panic::Location::caller();

    // If this is the first run and we are on the web client, the data might be cached
    #[cfg(feature = "web")]
    let initial_web_result =
        use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Some(storage_entry.get()))));

    let resource = use_resource(move || {
        #[cfg(feature = "server")]
        let storage_entry = storage_entry.clone();

        let user_fut = future();

        #[cfg(feature = "web")]
        let initial_web_result = initial_web_result.clone();

        #[allow(clippy::let_and_return)]
        async move {
            // If this is the first run and we are on the web client, the data might be cached
            #[cfg(feature = "web")]
            match initial_web_result.take() {
                // The data was deserialized successfully from the server
                Some(Ok(o)) => return o,

                // The data is still pending from the server. Don't try to resolve it on the client
                Some(Err(crate::TakeDataError::DataPending)) => std::future::pending::<()>().await,

                // The data was not available on the server, rerun the future
                Some(Err(_)) => {}

                // This isn't the first run, so we don't need do anything
                None => {}
            }

            // Otherwise just run the future itself
            let out = user_fut.await;

            // If this is the first run and we are on the server, cache the data in the slot we reserved for it
            #[cfg(feature = "server")]
            storage_entry.insert(&out, caller);

            out
        }
    });

    // On the first run, force this task to be polled right away in case its value is ready
    use_hook(|| {
        let _ = resource.task().poll_now();
    });

    // Suspend if the value isn't ready
    if resource.state().cloned() == UseResourceState::Pending {
        let task = resource.task();
        if !task.paused() {
            return Err(suspend(task).unwrap_err());
        }
    }

    Ok(resource)
}

// use dioxus_core::{suspend, use_hook, RenderError};
// use dioxus_hooks::*;
// use dioxus_signals::ReadableExt;
// use serde::{de::DeserializeOwned, Serialize};
// use std::future::Future;

// /// Runs a future with a manual list of dependencies and returns a resource with the result if the future is finished or a suspended error if it is still running.
// ///
// ///
// /// On the server, this will wait until the future is resolved before continuing to render. When the future is resolved, the result will be serialized into the page and hydrated on the client without rerunning the future.
// ///
// ///
// /// <div class="warning">
// ///
// /// Unlike [`use_resource`] dependencies are only tracked inside the function that spawns the async block, not the async block itself.
// ///
// /// ```rust, no_run
// /// # use dioxus::prelude::*;
// /// // ❌ The future inside of use_server_future is not reactive
// /// let id = use_signal(|| 0);
// /// use_server_future(move || {
// ///     async move {
// ///          // But the future is not reactive which means that the future will not subscribe to any reads here
// ///          println!("{id}");
// ///     }
// /// });
// /// // ✅ The closure that creates the future for use_server_future is reactive
// /// let id = use_signal(|| 0);
// /// use_server_future(move || {
// ///     // The closure itself is reactive which means the future will subscribe to any signals you read here
// ///     let cloned_id = id();
// ///     async move {
// ///          // But the future is not reactive which means that the future will not subscribe to any reads here
// ///          println!("{cloned_id}");
// ///     }
// /// });
// /// ```
// ///
// /// </div>
// ///
// /// # Example
// ///
// /// ```rust, no_run
// /// # use dioxus::prelude::*;
// /// # async fn fetch_article(id: u32) -> String { unimplemented!() }
// /// use dioxus::prelude::*;
// ///
// /// fn App() -> Element {
// ///     let mut article_id = use_signal(|| 0);
// ///     // `use_server_future` will spawn a task that runs on the server and serializes the result to send to the client.
// ///     // The future will rerun any time the
// ///     // Since we bubble up the suspense with `?`, the server will wait for the future to resolve before rendering
// ///     let article = use_server_future(move || fetch_article(article_id()))?;
// ///
// ///     rsx! {
// ///         "{article().unwrap()}"
// ///     }
// /// }
// /// ```
// #[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
// #[track_caller]
// pub fn use_server_future<T, F>(
//     mut future: impl FnMut() -> F + 'static,
// ) -> Result<Resource<T>, RenderError>
// where
//     T: Serialize + DeserializeOwned + 'static,
//     F: Future<Output = T> + 'static,
// {
//     let serialize_context = use_hook(crate::serialize_context);

//     // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
//     #[allow(unused)]
//     let storage_entry: crate::SerializeContextEntry<T> =
//         use_hook(|| serialize_context.create_entry());

//     #[cfg(feature = "server")]
//     let caller = std::panic::Location::caller();

//     // If this is the first run and we are on the web client, the data might be cached
//     #[cfg(feature = "web")]
//     let initial_web_result =
//         use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Some(storage_entry.get()))));

//     let resource = use_resource(move || {
//         #[cfg(feature = "server")]
//         let storage_entry = storage_entry.clone();

//         let user_fut = future();

//         #[cfg(feature = "web")]
//         let initial_web_result = initial_web_result.clone();

//         #[allow(clippy::let_and_return)]
//         async move {
//             // If this is the first run and we are on the web client, the data might be cached
//             #[cfg(feature = "web")]
//             match initial_web_result.take() {
//                 // The data was deserialized successfully from the server
//                 Some(Ok(o)) => return o,

//                 // The data is still pending from the server. Don't try to resolve it on the client
//                 Some(Err(crate::TakeDataError::DataPending)) => std::future::pending::<()>().await,

//                 // The data was not available on the server, rerun the future
//                 Some(Err(_)) => {}

//                 // This isn't the first run, so we don't need do anything
//                 None => {}
//             }

//             // Otherwise just run the future itself
//             let out = user_fut.await;

//             // If this is the first run and we are on the server, cache the data in the slot we reserved for it
//             #[cfg(feature = "server")]
//             storage_entry.insert(&out, caller);

//             out
//         }
//     });

//     // On the first run, force this task to be polled right away in case its value is ready
//     use_hook(|| {
//         let _ = resource.task().poll_now();
//     });

//     // Suspend if the value isn't ready
//     if resource.state().cloned() == UseResourceState::Pending {
//         let task = resource.task();
//         if !task.paused() {
//             return Err(suspend(task).unwrap_err());
//         }
//     }

//     Ok(resource)
// }

// // use dioxus_core::{suspend, use_hook, RenderError};
// // use dioxus_hooks::*;
// // use dioxus_signals::ReadableExt;
// // use std::future::Future;
// // // use crate::Transportable;

// // /// Runs a future with a manual list of dependencies and returns a resource with the result if the future is finished or a suspended error if it is still running.
// // ///
// // ///
// // /// On the server, this will wait until the future is resolved before continuing to render. When the future is resolved, the result will be serialized into the page and hydrated on the client without rerunning the future.
// // ///
// // ///
// // /// <div class="warning">
// // ///
// // /// Unlike [`use_resource`] dependencies are only tracked inside the function that spawns the async block, not the async block itself.
// // ///
// // /// ```rust, no_run
// // /// # use dioxus::prelude::*;
// // /// // ❌ The future inside of use_server_future is not reactive
// // /// let id = use_signal(|| 0);
// // /// use_server_future(move || {
// // ///     async move {
// // ///          // But the future is not reactive which means that the future will not subscribe to any reads here
// // ///          println!("{id}");
// // ///     }
// // /// });
// // /// // ✅ The closure that creates the future for use_server_future is reactive
// // /// let id = use_signal(|| 0);
// // /// use_server_future(move || {
// // ///     // The closure itself is reactive which means the future will subscribe to any signals you read here
// // ///     let cloned_id = id();
// // ///     async move {
// // ///          // But the future is not reactive which means that the future will not subscribe to any reads here
// // ///          println!("{cloned_id}");
// // ///     }
// // /// });
// // /// ```
// // ///
// // /// </div>
// // ///
// // /// # Example
// // ///
// // /// ```rust, no_run
// // /// # use dioxus::prelude::*;
// // /// # async fn fetch_article(id: u32) -> String { unimplemented!() }
// // /// use dioxus::prelude::*;
// // ///
// // /// fn App() -> Element {
// // ///     let mut article_id = use_signal(|| 0);
// // ///     // `use_server_future` will spawn a task that runs on the server and serializes the result to send to the client.
// // ///     // The future will rerun any time the
// // ///     // Since we bubble up the suspense with `?`, the server will wait for the future to resolve before rendering
// // ///     let article = use_server_future(move || fetch_article(article_id()))?;
// // ///
// // ///     rsx! {
// // ///         "{article().unwrap()}"
// // ///     }
// // /// }
// // /// ```
// // #[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
// // #[track_caller]
// // pub fn use_server_future<T, F, M>(
// //     future: impl FnMut() -> F + 'static,
// // ) -> Result<Resource<T>, RenderError>
// // where
// //     F: Future<Output = T> + 'static,
// //     T: Transportable<M>,
// //     M: 'static,
// // {
// //     let resource = use_server_future_unsuspended(future);

// //     // Suspend if the value isn't ready
// //     if resource.state().cloned() == UseResourceState::Pending {
// //         let task = resource.task();
// //         if !task.paused() {
// //             return Err(suspend(task).unwrap_err());
// //         }
// //     }

// //     Ok(resource)
// // }

// // /// Like [`use_server_future`] but does not suspend if the future is still running.
// // ///
// // /// You need to manage suspending yourself by checking the resource state.
// // pub fn use_server_future_unsuspended<T, F, M>(
// //     mut future: impl FnMut() -> F + 'static,
// // ) -> Resource<T>
// // where
// //     F: Future<Output = T> + 'static,
// //     T: Transportable<M>,
// //     M: 'static,
// // {
// //     let serialize_context = use_hook(crate::transport::serialize_context);

// //     // We always create a storage entry, even if the data isn't ready yet to make it possible to deserialize pending server futures on the client
// //     #[allow(unused)]
// //     let storage_entry: crate::transport::SerializeContextEntry<T> =
// //         use_hook(|| serialize_context.create_entry());

// //     #[cfg(feature = "server")]
// //     let caller = std::panic::Location::caller();

// //     // If this is the first run and we are on the web client, the data might be cached
// //     #[cfg(feature = "web")]
// //     let initial_web_result =
// //         use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Some(storage_entry.get()))));

// //     let resource = use_resource(move || {
// //         #[cfg(feature = "server")]
// //         let storage_entry = storage_entry.clone();

// //         let user_fut = future();

// //         #[cfg(feature = "web")]
// //         let initial_web_result = initial_web_result.clone();

// //         #[allow(clippy::let_and_return)]
// //         async move {
// //             // If this is the first run and we are on the web client, the data might be cached
// //             #[cfg(feature = "web")]
// //             match initial_web_result.take() {
// //                 // The data was deserialized successfully from the server
// //                 Some(Ok(o)) => return o,

// //                 // The data is still pending from the server. Don't try to resolve it on the client
// //                 Some(Err(crate::transport::TakeDataError::DataPending)) => {
// //                     std::future::pending::<()>().await
// //                 }

// //                 // The data was not available on the server, rerun the future
// //                 Some(Err(_)) => {}

// //                 // This isn't the first run, so we don't need do anything
// //                 None => {}
// //             }

// //             // Otherwise just run the future itself
// //             let out = user_fut.await;

// //             // If this is the first run and we are on the server, cache the data in the slot we reserved for it
// //             #[cfg(feature = "server")]
// //             storage_entry.insert(&out, caller);

// //             out
// //         }
// //     });

// //     // On the first run, force this task to be polled right away in case its value is ready
// //     use_hook(|| {
// //         let _ = resource.task().poll_now();
// //     });

// //     resource
// // }
