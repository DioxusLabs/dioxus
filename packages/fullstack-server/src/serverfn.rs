use axum::routing::MethodRouter;
use dioxus_fullstack_core::ServerFnState;
use http::Method;

/// A function endpoint that can be called from the client.
#[derive(Clone)]
pub struct ServerFunction {
    path: &'static str,
    method: Method,
    handler: fn() -> MethodRouter<ServerFnState>,
}

impl ServerFunction {
    /// Create a new server function object.
    pub const fn new(
        method: Method,
        path: &'static str,
        handler: fn() -> MethodRouter<ServerFnState>,
    ) -> Self {
        Self {
            path,
            method,
            handler,
        }
    }

    /// The path of the server function.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// The HTTP method the server function expects.
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// Collect all globally registered server functions
    pub fn collect() -> Vec<&'static ServerFunction> {
        inventory::iter::<ServerFunction>().collect()
    }

    /// The handler for the server function. Note that this cannot be used directly since it does
    /// not included the required middleware to make `ServerFnState` populated nor `FullstackContext`
    /// available via tokio's `task_local`
    pub fn handler(&self) -> MethodRouter<ServerFnState> {
        (self.handler)()
    }
}

impl inventory::Collect for ServerFunction {
    #[inline]
    fn registry() -> &'static inventory::Registry {
        static REGISTRY: inventory::Registry = inventory::Registry::new();
        &REGISTRY
    }
}
