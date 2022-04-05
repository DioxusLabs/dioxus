#[derive(Clone, Copy)]
pub struct Config {
    pub rendering_mode: RenderingMode,
    /// Controls if the terminal quit when the user presses `ctrl+c`?
    /// To handle quiting on your own, use the [crate::TuiContext] root context.
    pub ctrl_c_quit: bool,
    /// Controls if the terminal should dislay anything, usefull for testing.
    pub headless: bool,
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
