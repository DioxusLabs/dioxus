use std::future::Future;

use dioxus_core::{Element, VirtualDom};

#[cfg(feature = "axum")]
pub mod axum_adapter;
#[cfg(feature = "axum")]
pub use axum_adapter::*;

/// A trait for servers that can be used to host a LiveView app.
pub trait LiveviewRouter {
    /// Create a new router.
    fn create_default_liveview_router() -> Self;

    /// Add a liveview route to the server from a component
    fn with_app(self, route: &str, app: fn() -> Element) -> Self
    where
        Self: Sized,
    {
        self.with_virtual_dom(route, move || VirtualDom::new(app))
    }

    /// Add a liveview route to the server from a virtual dom.
    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) -> Self;

    /// Start the server on an address.
    fn start(self, address: impl Into<std::net::SocketAddr>) -> impl Future<Output = ()>;
}
