use std::ops::{Deref, DerefMut};

use dioxus_core::DomEdit;
use wry::{
    application::{
        error::OsError,
        event_loop::{EventLoop, EventLoopWindowTarget},
        menu::MenuBar,
        window::{Fullscreen, Icon, Window, WindowBuilder},
    },
    webview::{RpcRequest, RpcResponse, WebView},
};

pub struct DesktopConfig<'a> {
    pub window: WindowBuilder,
    pub(crate) manual_edits: Option<Vec<DomEdit<'a>>>,
    pub(crate) pre_rendered: Option<String>,
    pub(crate) event_handler: Option<Box<dyn Fn(&mut EventLoop<()>, &mut WebView)>>,
}

impl<'a> DesktopConfig<'a> {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        let mut window = WindowBuilder::new().with_title("Dioxus app");
        Self {
            event_handler: None,
            window,
            pre_rendered: None,
            manual_edits: None,
        }
    }

    pub fn with_edits(&mut self, edits: Vec<DomEdit<'a>>) -> &mut Self {
        self.manual_edits = Some(edits);
        self
    }

    pub fn with_prerendered(&mut self, content: String) -> &mut Self {
        self.pre_rendered = Some(content);
        self
    }

    pub fn with_window(&mut self, f: impl FnOnce(WindowBuilder) -> WindowBuilder) -> &mut Self {
        // gots to do a swap because the window builder only takes itself as muy self
        // I wish more people knew about returning &mut Self
        let mut builder = WindowBuilder::default().with_title("Dioxus App");
        std::mem::swap(&mut self.window, &mut builder);
        builder = f(builder);
        std::mem::swap(&mut self.window, &mut builder);
        self
    }
}
