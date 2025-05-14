use dioxus_core::prelude::use_hook;
use dioxus_fullstack_protocol::SerializeContextEntry;
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
///    unimplemented!()
/// }
/// ```
#[track_caller]
pub fn use_server_cached<O: 'static + Clone + Serialize + DeserializeOwned>(
    server_fn: impl Fn() -> O,
) -> O {
    let location = std::panic::Location::caller();
    use_hook(|| server_cached(server_fn, location))
}

pub(crate) fn server_cached<O: 'static + Clone + Serialize + DeserializeOwned>(
    value: impl FnOnce() -> O,
    #[allow(unused)] location: &'static std::panic::Location<'static>,
) -> O {
    let serialize = dioxus_fullstack_protocol::serialize_context();
    #[allow(unused)]
    let entry: SerializeContextEntry<O> = serialize.create_entry();
    #[cfg(feature = "server")]
    {
        let data = value();
        entry.insert(&data, location);
        data
    }
    #[cfg(all(not(feature = "server"), feature = "web"))]
    {
        match entry.get() {
            Ok(value) => value,
            Err(_) => value(),
        }
    }
    #[cfg(not(any(feature = "server", feature = "web")))]
    {
        value()
    }
}
