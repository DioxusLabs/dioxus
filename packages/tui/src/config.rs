#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Config {
    pub(crate) rendering_mode: RenderingMode,
    /// Controls if the terminal quit when the user presses `ctrl+c`?
    /// To handle quiting on your own, use the [crate::TuiContext] root context.
    pub(crate) ctrl_c_quit: bool,
    /// Controls if the terminal should dislay anything, usefull for testing.
    pub(crate) headless: bool,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rendering_mode(self, rendering_mode: RenderingMode) -> Self {
        Self {
            rendering_mode,
            ..self
        }
    }

    pub fn with_ctrl_c_quit(self, ctrl_c_quit: bool) -> Self {
        Self {
            ctrl_c_quit,
            ..self
        }
    }

    pub fn with_headless(self, headless: bool) -> Self {
        Self { headless, ..self }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rendering_mode: Default::default(),
            ctrl_c_quit: true,
            headless: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum RenderingMode {
    /// only 16 colors by accessed by name, no alpha support
    BaseColors,
    /// 8 bit colors, will be downsampled from rgb colors
    Ansi,
    /// 24 bit colors, most terminals support this
    Rgb,
}

impl Default for RenderingMode {
    fn default() -> Self {
        RenderingMode::Rgb
    }
}
