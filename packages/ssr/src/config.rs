#[derive(Clone, Debug, Default)]
pub struct SsrConfig {
    /// currently not supported - control if we indent the HTML output
    indent: bool,

    /// Control if elements are written onto a new line
    newline: bool,

    /// Choose to write ElementIDs into elements so the page can be re-hydrated later on
    pre_render: bool,

    // Currently not implemented
    // Don't proceed onto new components. Instead, put the name of the component.
    // TODO: components don't have names :(
    skip_components: bool,
}

impl SsrConfig {
    pub fn indent(mut self, a: bool) -> Self {
        self.indent = a;
        self
    }

    pub fn newline(mut self, a: bool) -> Self {
        self.newline = a;
        self
    }

    pub fn pre_render(mut self, a: bool) -> Self {
        self.pre_render = a;
        self
    }

    pub fn skip_components(mut self, a: bool) -> Self {
        self.skip_components = a;
        self
    }
}
