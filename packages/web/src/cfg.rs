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
    pub(crate) root: ConfigRoot,
    pub(crate) default_panic_hook: bool,
}

pub(crate) enum ConfigRoot {
    RootName(String),
    RootElement(web_sys::Element),
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
    /// Note that this only works on the current document, i.e. `window.document`.
    /// To use a different document (popup, iframe, ...) use [Self::rootelement] instead.
    pub fn rootname(mut self, name: impl Into<String>) -> Self {
        self.root = ConfigRoot::RootName(name.into());
        self
    }

    /// Set the element that Dioxus will use as root.
    ///
    /// This is akin to calling React.render() on the given element.
    pub fn rootelement(mut self, elem: web_sys::Element) -> Self {
        self.root = ConfigRoot::RootElement(elem);
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

impl Default for Config {
    fn default() -> Self {
        Self {
            hydrate: false,
            root: ConfigRoot::RootName("main".to_string()),
            default_panic_hook: true,
        }
    }
}
