use crate::{transport::SerializeContextEntry, Transportable};
use dioxus_core::use_hook;

/// This allows you to send data from the server to the client *during hydration*.
/// - When compiled as server, the closure is ran and the resulting data is serialized on the server and sent to the client.
/// - When compiled as web client, the data is deserialized from the server if already available, otherwise runs on the client. Data is usually only available if this hook exists in a component during hydration.
/// - When otherwise compiled, the closure is run directly with no serialization.
///
/// The order this function is run on the client needs to be the same order initially run on the server.
///
/// If Dioxus fullstack cannot find the data on the client, it will run the closure again to get the data.
///
/// # Example
/// ```rust
/// use dioxus::prelude::*;
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
pub fn use_server_cached<O, M>(server_fn: impl Fn() -> O) -> O
where
    O: Transportable<M> + Clone,
    M: 'static,
{
    let location = std::panic::Location::caller();
    use_hook(|| server_cached(server_fn, location))
}

pub(crate) fn server_cached<O, M>(
    value: impl FnOnce() -> O,
    #[allow(unused)] location: &'static std::panic::Location<'static>,
) -> O
where
    O: Transportable<M> + Clone,
    M: 'static,
{
    let serialize = crate::transport::serialize_context();

    #[allow(unused)]
    let entry: SerializeContextEntry<O> = serialize.create_entry();

    // Use target_arch instead of cfg(feature) because cargo feature unification
    // can enable both "web" and "server" features simultaneously.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let data = value();
        entry.insert(&data, location);
        data
    }

    #[cfg(target_arch = "wasm32")]
    {
        match entry.get() {
            Ok(value) => value,
            Err(_) => value(),
        }
    }
}
