pub struct WebConfig {
    pub(crate) hydrate: bool,
}
impl Default for WebConfig {
    fn default() -> Self {
        Self { hydrate: false }
    }
}
impl WebConfig {
    /// Enable SSR hydration
    ///
    /// This enables Dioxus to pick up work from a pre-renderd HTML file. Hydration will completely skip over any async
    /// work and suspended nodes.
    ///
    /// Dioxus will load up all the elements with the `dio_el` data attribute into memory when the page is loaded.
    ///
    pub fn hydrate(mut self, f: bool) -> Self {
        self.hydrate = f;
        self
    }
}
