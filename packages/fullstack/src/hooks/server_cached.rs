use dioxus_lib::prelude::use_hook;
use serde::{de::DeserializeOwned, Serialize};

/// This allows you to send data from the server to the client. The data is serialized into the HTML on the server and hydrated on the client.
///
/// When you run this function on the client, you need to be careful to insure the order you run it initially is the same order you run it on the server.
///
/// If Dioxus fullstack cannot find the data on the client, it will run the closure again to get the data.
///
/// # Example
/// ```rust
/// use dioxus_lib::prelude::*;
/// use dioxus_fullstack::prelude::*;
///
/// fn app() -> Element {
///    let state1 = use_server_cached(|| {
///       1234
///    });
///
///    todo!()
/// }
/// ```
pub fn use_server_cached<O: 'static + Clone + Serialize + DeserializeOwned>(
    server_fn: impl Fn() -> O,
) -> O {
    use_hook(|| server_cached(server_fn))
}

pub(crate) fn server_cached<O: 'static + Clone + Serialize + DeserializeOwned>(
    value: impl FnOnce() -> O,
) -> O {
    #[cfg(feature = "server")]
    {
        let serialize = crate::html_storage::serialize_context();
        let data = value();
        serialize.push(&data);
        data
    }
    #[cfg(all(not(feature = "server"), feature = "web"))]
    {
        dioxus_web::take_server_data()
            .ok()
            .flatten()
            .unwrap_or_else(value)
    }
    #[cfg(not(any(feature = "server", feature = "web")))]
    {
        value()
    }
}
