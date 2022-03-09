#[derive(Default, Clone, Copy)]
pub struct Config {
    pub rendering_mode: RenderingMode,
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
