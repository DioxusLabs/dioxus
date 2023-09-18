use serde::{de::DeserializeOwned, Serialize};

/// This allows you to send data from the server to the client. The data is serialized into the HTML on the server and hydrated on the client.
///
/// When you run this function on the client, you need to be careful to insure the order you run it initially is the same order you run it on the server.
///
/// If Dioxus fullstack cannot find the data on the client, it will run the closure again to get the data.
///
/// # Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_fullstack::prelude::*;
///
/// fn app(cx: Scope) -> Element {
///    let state1 = use_state(cx, || server_cached(|| {
///       1234
///    }));
///    todo!()
/// }
/// ```
pub fn server_cached<O: 'static + Serialize + DeserializeOwned>(server_fn: impl Fn() -> O) -> O {
    #[cfg(feature = "ssr")]
    {
        let data = server_fn();
        let sc = crate::prelude::server_context();
        if let Err(err) = sc.push_html_data(&data) {
            tracing::error!("Failed to push HTML data: {}", err);
        }
        data
    }
    #[cfg(not(feature = "ssr"))]
    {
        crate::html_storage::deserialize::take_server_data().unwrap_or_else(server_fn)
    }
}
