mod adapters;

// #[server(ReadPosts, "api")]
// async fn testing(rx: i32) -> Result<u32, ServerFnError> {
//     Ok(0)
// }

pub struct DioxusServerContext {}

#[cfg(any(feature = "ssr", doc))]
type ServerFnTraitObj = server_fn::ServerFnTraitObj<DioxusServerContext>;

#[cfg(any(feature = "ssr", doc))]
static REGISTERED_SERVER_FUNCTIONS: once_cell::sync::Lazy<
    std::sync::Arc<
        std::sync::RwLock<
            std::collections::HashMap<&'static str, std::sync::Arc<ServerFnTraitObj>>,
        >,
    >,
> = once_cell::sync::Lazy::new(Default::default);

#[cfg(any(feature = "ssr", doc))]
/// The registry of all Dioxus server functions.
pub struct DioxusServerFnRegistry;

#[cfg(any(feature = "ssr"))]
impl server_fn::ServerFunctionRegistry<DioxusServerContext> for DioxusServerFnRegistry {
    type Error = ServerRegistrationFnError;

    fn register(
        url: &'static str,
        server_function: std::sync::Arc<ServerFnTraitObj>,
    ) -> Result<(), Self::Error> {
        // store it in the hashmap
        let mut write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerRegistrationFnError::Poisoned(e.to_string()))?;
        let prev = write.insert(url, server_function);

        // if there was already a server function with this key,
        // return Err
        match prev {
            Some(_) => Err(ServerRegistrationFnError::AlreadyRegistered(format!(
                "There was already a server function registered at {:?}. \
                     This can happen if you use the same server function name \
                     in two different modules
                on `stable` or in `release` mode.",
                url
            ))),
            None => Ok(()),
        }
    }

    /// Returns the server function registered at the given URL, or `None` if no function is registered at that URL.
    fn get(url: &str) -> Option<std::sync::Arc<ServerFnTraitObj>> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).cloned())
    }

    /// Returns a list of all registered server functions.
    fn paths_registered() -> Vec<&'static str> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .map(|fns| fns.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(any(feature = "ssr", doc))]
/// Errors that can occur when registering a server function.
#[derive(thiserror::Error, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerRegistrationFnError {
    /// The server function is already registered.
    #[error("The server function {0} is already registered")]
    AlreadyRegistered(String),
    /// The server function registry is poisoned.
    #[error("The server function registry is poisoned: {0}")]
    Poisoned(String),
}

/// Defines a "server function." A server function can be called from the server or the client,
/// but the body of its code will only be run on the server, i.e., if a crate feature `ssr` is enabled.
///
/// (This follows the same convention as the Dioxus framework's distinction between `ssr` for server-side rendering,
/// and `csr` and `hydrate` for client-side rendering and hydration, respectively.)
///
/// Server functions are created using the `server` macro.
///
/// The function should be registered by calling `ServerFn::register()`. The set of server functions
/// can be queried on the server for routing purposes by calling [server_fn_by_path].
///
/// Technically, the trait is implemented on a type that describes the server function's arguments.
pub trait ServerFn: server_fn::ServerFn<DioxusServerContext> {
    /// Registers the server function, allowing the server to query it by URL.
    #[cfg(any(feature = "ssr", doc))]
    fn register() -> Result<(), server_fn::ServerFnError> {
        Self::register_in::<DioxusServerFnRegistry>()
    }
}

impl<T> ServerFn for T where T: server_fn::ServerFn<DioxusServerContext> {}
