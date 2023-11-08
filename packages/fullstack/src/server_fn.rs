#[cfg(any(feature = "ssr", doc))]
#[derive(Clone)]
/// A trait object for a function that be called on serializable arguments and returns a serializable result.
pub struct ServerFnTraitObj(server_fn::ServerFnTraitObj<()>);

#[cfg(any(feature = "ssr", doc))]
impl std::ops::Deref for ServerFnTraitObj {
    type Target = server_fn::ServerFnTraitObj<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(any(feature = "ssr", doc))]
impl std::ops::DerefMut for ServerFnTraitObj {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(any(feature = "ssr", doc))]
impl ServerFnTraitObj {
    fn new(
        prefix: &'static str,
        url: &'static str,
        encoding: server_fn::Encoding,
        run: ServerFunction,
    ) -> Self {
        Self(server_fn::ServerFnTraitObj::new(prefix, url, encoding, run))
    }

    /// Create a new `ServerFnTraitObj` from a `server_fn::ServerFnTraitObj`.
    pub const fn from_generic_server_fn(server_fn: server_fn::ServerFnTraitObj<()>) -> Self {
        Self(server_fn)
    }
}

#[cfg(feature = "ssr")]
server_fn::inventory::collect!(ServerFnTraitObj);

#[cfg(feature = "ssr")]
/// Middleware for a server function
pub struct ServerFnMiddleware {
    /// The prefix of the server function.
    pub prefix: &'static str,
    /// The url of the server function.
    pub url: &'static str,
    /// The middleware layers.
    pub middleware: fn() -> Vec<std::sync::Arc<dyn crate::layer::Layer>>,
}

#[cfg(feature = "ssr")]
pub(crate) static MIDDLEWARE: once_cell::sync::Lazy<
    std::collections::HashMap<
        (&'static str, &'static str),
        Vec<std::sync::Arc<dyn crate::layer::Layer>>,
    >,
> = once_cell::sync::Lazy::new(|| {
    let mut map: std::collections::HashMap<
        (&'static str, &'static str),
        Vec<std::sync::Arc<dyn crate::layer::Layer>>,
    > = std::collections::HashMap::new();
    for middleware in server_fn::inventory::iter::<ServerFnMiddleware> {
        map.entry((middleware.prefix, middleware.url))
            .or_default()
            .extend((middleware.middleware)().iter().cloned());
    }
    map
});

#[cfg(feature = "ssr")]
server_fn::inventory::collect!(ServerFnMiddleware);

#[cfg(any(feature = "ssr", doc))]
/// A server function that can be called on serializable arguments and returns a serializable result.
pub type ServerFunction = server_fn::SerializedFnTraitObj<()>;

#[cfg(feature = "ssr")]
#[allow(clippy::type_complexity)]
static REGISTERED_SERVER_FUNCTIONS: once_cell::sync::Lazy<
    std::sync::Arc<std::sync::RwLock<std::collections::HashMap<&'static str, ServerFnTraitObj>>>,
> = once_cell::sync::Lazy::new(|| {
    let mut map = std::collections::HashMap::new();
    for server_fn in server_fn::inventory::iter::<ServerFnTraitObj> {
        map.insert(server_fn.0.url(), server_fn.clone());
    }
    std::sync::Arc::new(std::sync::RwLock::new(map))
});

#[cfg(any(feature = "ssr", doc))]
/// The registry of all Dioxus server functions.
pub struct DioxusServerFnRegistry;

#[cfg(feature = "ssr")]
impl server_fn::ServerFunctionRegistry<()> for DioxusServerFnRegistry {
    type Error = ServerRegistrationFnError;

    fn register_explicit(
        prefix: &'static str,
        url: &'static str,
        server_function: ServerFunction,
        encoding: server_fn::Encoding,
    ) -> Result<(), Self::Error> {
        // store it in the hashmap
        let mut write = REGISTERED_SERVER_FUNCTIONS
            .write()
            .map_err(|e| ServerRegistrationFnError::Poisoned(e.to_string()))?;
        let prev = write.insert(
            url,
            ServerFnTraitObj::new(prefix, url, encoding, server_function),
        );

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
    fn get(url: &str) -> Option<server_fn::ServerFnTraitObj<()>> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|inner| inner.0.clone()))
    }

    /// Returns the server function registered at the given URL, or `None` if no function is registered at that URL.
    fn get_trait_obj(url: &str) -> Option<server_fn::ServerFnTraitObj<()>> {
        Self::get(url)
    }

    fn get_encoding(url: &str) -> Option<server_fn::Encoding> {
        REGISTERED_SERVER_FUNCTIONS
            .read()
            .ok()
            .and_then(|fns| fns.get(url).map(|f| f.encoding()))
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
/// Server functions are created using the `server` macro.
///
/// The set of server functions
/// can be queried on the server for routing purposes by calling [server_fn::ServerFunctionRegistry::get].
///
/// Technically, the trait is implemented on a type that describes the server function's arguments, not the function itself.
pub trait DioxusServerFn: server_fn::ServerFn<()> {
    /// Registers the server function, allowing the client to query it by URL.
    #[cfg(any(feature = "ssr", doc))]
    fn register_explicit() -> Result<(), server_fn::ServerFnError> {
        Self::register_in_explicit::<DioxusServerFnRegistry>()
    }
}

impl<T> DioxusServerFn for T where T: server_fn::ServerFn<()> {}
