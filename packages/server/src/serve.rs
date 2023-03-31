use dioxus_core::Component;

#[derive(Clone)]
pub struct ServeConfig<P: Clone> {
    pub(crate) app: Component<P>,
    pub(crate) props: P,
    pub(crate) application_name: Option<&'static str>,
    pub(crate) server_fn_route: Option<&'static str>,
    pub(crate) base_path: Option<&'static str>,
    pub(crate) head: Option<&'static str>,
}

impl<P: Clone> ServeConfig<P> {
    /// Create a new ServeConfig
    pub fn new(app: Component<P>, props: P) -> Self {
        Self {
            app,
            props,
            application_name: None,
            server_fn_route: None,
            base_path: None,
            head: None,
        }
    }

    /// Set the application name matching the name in the Dioxus.toml file used to build the application
    pub fn application_name(mut self, application_name: &'static str) -> Self {
        self.application_name = Some(application_name);
        self
    }

    /// Set the base route all server functions will be served under
    pub fn server_fn_route(mut self, server_fn_route: &'static str) -> Self {
        self.server_fn_route = Some(server_fn_route);
        self
    }

    /// Set the path the WASM application will be served under
    pub fn base_path(mut self, base_path: &'static str) -> Self {
        self.base_path = Some(base_path);
        self
    }

    /// Set the head content to be included in the HTML document served
    pub fn head(mut self, head: &'static str) -> Self {
        self.head = Some(head);
        self
    }
}
