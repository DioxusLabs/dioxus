use dioxus_core::LaunchConfig;
use winit::window::WindowAttributes;

/// The configuration for the desktop application.
pub struct Config {
    pub(crate) window_attributes: WindowAttributes,
}

impl LaunchConfig for Config {}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_attributes: WindowAttributes::default().with_title(
                dioxus_cli_config::app_title().unwrap_or_else(|| "Dioxus App".to_string()),
            ),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration for the window.
    pub fn with_window_attributes(mut self, attrs: WindowAttributes) -> Self {
        // We need to do a swap because the window builder only takes itself as muy self
        self.window_attributes = attrs;
        self
    }
}
