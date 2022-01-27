use wry::{
    application::{
        event_loop::EventLoop,
        window::{Window, WindowBuilder},
    },
    http::{Request as HttpRequest, Response as HttpResponse},
    webview::{FileDropEvent, WebView},
    Result as WryResult,
};

pub(crate) type DynEventHandlerFn = dyn Fn(&mut EventLoop<()>, &mut WebView);

pub struct DesktopConfig {
    pub window: WindowBuilder,
    pub file_drop_handler: Option<Box<dyn Fn(&Window, FileDropEvent) -> bool>>,
    pub protocos: Vec<WryProtocl>,
    pub(crate) pre_rendered: Option<String>,
    pub(crate) event_handler: Option<Box<DynEventHandlerFn>>,
}

pub type WryProtocl = (
    String,
    Box<dyn Fn(&HttpRequest) -> WryResult<HttpResponse> + 'static>,
);

impl DesktopConfig {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        let window = WindowBuilder::new().with_title("Dioxus app");
        Self {
            event_handler: None,
            window,
            protocos: Vec::new(),
            file_drop_handler: None,
            pre_rendered: None,
        }
    }

    pub fn with_prerendered(&mut self, content: String) -> &mut Self {
        self.pre_rendered = Some(content);
        self
    }

    pub fn with_window(
        &mut self,
        configure: impl FnOnce(WindowBuilder) -> WindowBuilder,
    ) -> &mut Self {
        // gots to do a swap because the window builder only takes itself as muy self
        // I wish more people knew about returning &mut Self
        let mut builder = WindowBuilder::default().with_title("Dioxus App");
        std::mem::swap(&mut self.window, &mut builder);
        builder = configure(builder);
        std::mem::swap(&mut self.window, &mut builder);
        self
    }

    pub fn with_event_handler(
        &mut self,
        handler: impl Fn(&mut EventLoop<()>, &mut WebView) + 'static,
    ) -> &mut Self {
        self.event_handler = Some(Box::new(handler));
        self
    }

    pub fn with_file_drop_handler(
        &mut self,
        handler: impl Fn(&Window, FileDropEvent) -> bool + 'static,
    ) -> &mut Self {
        self.file_drop_handler = Some(Box::new(handler));
        self
    }

    pub fn with_custom_protocol<F>(&mut self, name: String, handler: F) -> &mut Self
    where
        F: Fn(&HttpRequest) -> WryResult<HttpResponse> + 'static,
    {
        self.protocos.push((name, Box::new(handler)));
        self
    }
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self::new()
    }
}
