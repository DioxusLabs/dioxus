use dioxus_core::{LaunchConfig, VirtualDom};

use crate::LiveviewRouter;

pub(crate) fn app_title() -> String {
    dioxus_cli_config::app_title().unwrap_or_else(|| "Dioxus Liveview App".to_string())
}

/// A configuration for the LiveView server.
pub struct Config<R: LiveviewRouter> {
    router: R,
    address: std::net::SocketAddr,
    route: String,
}

impl<R: LiveviewRouter + 'static> LaunchConfig for Config<R> {}

impl<R: LiveviewRouter> Default for Config<R> {
    fn default() -> Self {
        Self {
            address: dioxus_cli_config::fullstack_address_or_localhost(),
            router: R::create_default_liveview_router(),
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
    pub fn with_app(mut self, app: fn() -> dioxus_core::Element) -> Self {
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
        println!("{} started on http://{}", app_title(), self.address);
        self.router.start(self.address).await
    }
}
