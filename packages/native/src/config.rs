use anyrender::CompositeAlphaMode;
use blitz_dom::FontContext;
use dioxus_core::LaunchConfig;
use peniko::Color;
use winit::window::WindowAttributes;

/// Launch-time configuration for a dioxus-native application.
pub struct Config {
    pub(crate) window_attributes: WindowAttributes,
    pub(crate) font_ctx: Option<FontContext>,
    pub(crate) alpha_mode: Option<CompositeAlphaMode>,
    pub(crate) base_color: Option<Color>,
}

impl LaunchConfig for Config {}

impl Default for Config {
    fn default() -> Self {
        let window_attributes = WindowAttributes::default()
            .with_title(dioxus_cli_config::app_title().unwrap_or_else(|| "Dioxus App".to_string()));

        // On WASM, append the canvas to the document body. Surface size is
        // intentionally not seeded: winit-web's with_surface_size writes inline
        // canvas.style.width/height that overrides host CSS. View::init reads
        // the canvas's CSS layout box when winit reports a 0×0 initial size.
        // To target an existing `<canvas>`, replace these attributes via
        // [`Config::with_window_attributes`].
        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            use winit::platform::web::WindowAttributesWeb;
            window_attributes.with_platform_attributes(Box::new(
                WindowAttributesWeb::default().with_append(true),
            ))
        };

        Self {
            window_attributes,
            font_ctx: None,
            alpha_mode: None,
            base_color: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration for the window.
    pub fn with_window_attributes(mut self, attrs: WindowAttributes) -> Self {
        self.window_attributes = attrs;
        self
    }

    /// Set a custom [`FontContext`] for the document.
    ///
    /// On WASM, browsers don't expose system fonts, so a `FontContext` with
    /// bundled fonts must be provided. Use [`crate::build_single_font_ctx`] for
    /// the common one-font case.
    pub fn with_font_ctx(mut self, font_ctx: FontContext) -> Self {
        self.font_ctx = Some(font_ctx);
        self
    }

    /// Set the alpha mode used when compositing the window surface.
    ///
    /// This controls how the rendered content is blended with the background,
    /// e.g. to enable transparent windows. Not all renderers support every
    /// mode; unsupported modes are ignored by the active renderer.
    pub fn with_alpha_mode(mut self, alpha_mode: CompositeAlphaMode) -> Self {
        self.alpha_mode = Some(alpha_mode);
        self
    }

    /// Set the base (background) color used to clear each frame.
    pub fn with_base_color(mut self, base_color: Color) -> Self {
        self.base_color = Some(base_color);
        self
    }
}
