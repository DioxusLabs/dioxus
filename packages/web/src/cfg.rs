///  Configuration for the WebSys renderer for the Dioxus VirtualDOM.
///
/// This struct helps configure the specifics of hydration and render destination for WebSys.
///
/// # Example
///
/// ```rust, ignore
/// dioxus_web::launch(App, |cfg| cfg.hydrate(true).root_name("myroot"))
/// ```
pub struct WebConfig {
    pub(crate) hydrate: bool,
    pub(crate) rootname: String,
    pub(crate) cached_strings: Vec<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            hydrate: false,
            rootname: "main".to_string(),
            cached_strings: Vec::new(),
        }
    }
}

impl WebConfig {
    /// Enable SSR hydration
    ///
    /// This enables Dioxus to pick up work from a pre-renderd HTML file. Hydration will completely skip over any async
    /// work and suspended nodes.
    ///
    /// Dioxus will load up all the elements with the `dio_el` data attribute into memory when the page is loaded.
    pub fn hydrate(&mut self, f: bool) -> &mut Self {
        self.hydrate = f;
        self
    }

    /// Set the name of the element that Dioxus will use as the root.
    ///
    /// This is akint to calling React.render() on the element with the specified name.
    pub fn rootname(&mut self, name: impl Into<String>) -> &mut Self {
        self.rootname = name.into();
        self
    }

    /// Set the name of the element that Dioxus will use as the root.
    ///
    /// This is akint to calling React.render() on the element with the specified name.
    pub fn with_string_cache(&mut self, cache: Vec<String>) -> &mut Self {
        self.cached_strings = cache;
        self
    }
}
