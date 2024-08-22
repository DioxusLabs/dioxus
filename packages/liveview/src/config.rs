use dioxus_core::VirtualDom;

use crate::LiveviewRouter;

/// A configuration for the LiveView server.
pub struct Config<R: LiveviewRouter> {
    router: R,
    address: std::net::SocketAddr,
    route: String,
}

impl<R: LiveviewRouter> Default for Config<R> {
    fn default() -> Self {
        let address = dioxus_runtime_config::fullstack_address().unwrap_or_else(|| {
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                8080,
            ))
        });

        Self {
            router: R::create_default_liveview_router(),
            address,
            route: "/".to_string(),
        }
    }
}

impl<R: LiveviewRouter> Config<R> {
    /// Set the route to use for the LiveView server.
    pub fn route(mut self, route: impl Into<String>) -> Self {
        self.route = route.into();
        self
    }

    /// Create a new configuration for the LiveView server.
    pub fn with_app(mut self, app: fn() -> dioxus_core::prelude::Element) -> Self {
        self.router = self.router.with_app(&self.route, app);
        self
    }

    /// Create a new configuration for the LiveView server.
    pub fn with_virtual_dom(
        mut self,
        virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    ) -> Self {
        self.router = self.router.with_virtual_dom(&self.route, virtual_dom);
        self
    }

    /// Set the address to listen on.
    pub fn address(mut self, address: impl Into<std::net::SocketAddr>) -> Self {
        self.address = address.into();
        self
    }

    /// Launch the LiveView server.
    pub async fn launch(self) {
        println!("Listening on http://{}", self.address);
        self.router.start(self.address).await
    }
}
