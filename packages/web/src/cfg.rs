///  Configuration for the WebSys renderer for the Dioxus VirtualDOM.
///
/// This struct helps configure the specifics of hydration and render destination for WebSys.
///
/// # Example
///
/// ```rust, ignore
/// dioxus_web::launch(App, Config::new().hydrate(true).root_name("myroot"))
/// ```
pub struct Config {
    pub(crate) hydrate: bool,
    pub(crate) rootname: String,
    pub(crate) default_panic_hook: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hydrate: false,
            rootname: "main".to_string(),
            default_panic_hook: true,
        }
    }
}

impl Config {
    /// Create a new Default instance of the Config.
    ///
    /// This is no different than calling `Config::default()`
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "hydrate")]
    /// Enable SSR hydration
    ///
    /// This enables Dioxus to pick up work from a pre-renderd HTML file. Hydration will completely skip over any async
    /// work and suspended nodes.
    ///
    /// Dioxus will load up all the elements with the `dio_el` data attribute into memory when the page is loaded.
    pub fn hydrate(mut self, f: bool) -> Self {
        self.hydrate = f;
        self
    }

    /// Set the name of the element that Dioxus will use as the root.
    ///
    /// This is akin to calling React.render() on the element with the specified name.
    pub fn rootname(mut self, name: impl Into<String>) -> Self {
        self.rootname = name.into();
        self
    }

    /// Set whether or not Dioxus should use the built-in panic hook or defer to your own.
    ///
    /// The panic hook is set to true normally so even the simplest apps have helpful error messages.
    pub fn with_default_panic_hook(mut self, f: bool) -> Self {
        self.default_panic_hook = f;
        self
    }
}
