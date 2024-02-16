#[cfg(feature = "server")]
pub(crate)mod collection;
#[cfg(feature = "server")]
pub mod service;

/// Defines a "server function." A server function can be called from the server or the client,
/// but the body of its code will only be run on the server, i.e., if a crate feature `ssr` is enabled.
///
/// Server functions are created using the `server` macro.
///
/// The set of server functions
/// can be queried on the server for routing purposes by calling [server_fn::ServerFunctionRegistry::get].
///
/// Technically, the trait is implemented on a type that describes the server function's arguments, not the function itself.
pub trait DioxusServerFn: server_fn::ServerFn<()> {
    /// Registers the server function, allowing the client to query it by URL.
    #[cfg(feature = "server")]
    #[cfg_attr(docsrs, doc(cfg(feature = "server")))]
    fn register_explicit() -> Result<(), server_fn::ServerFnError> {
        Self::register_in_explicit::<crate::server_fn::collection::DioxusServerFnRegistry>()
    }
}

impl<T> DioxusServerFn for T where T: server_fn::ServerFn<()> {}
