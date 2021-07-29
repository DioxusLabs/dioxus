use std::ops::{Deref, DerefMut};

use dioxus_core::DomEdit;
use wry::{
    application::{
        error::OsError,
        event_loop::EventLoopWindowTarget,
        menu::MenuBar,
        window::{Fullscreen, Icon, Window, WindowBuilder},
    },
    webview::{RpcRequest, RpcResponse},
};

pub struct DesktopConfig<'a> {
    pub window: WindowBuilder,
    pub(crate) manual_edits: Option<DomEdit<'a>>,
    pub(crate) pre_rendered: Option<String>,
}

impl DesktopConfig<'_> {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        Self {
            window: Default::default(),
            pre_rendered: None,
            manual_edits: None,
        }
    }

    pub fn with_prerendered(&mut self, content: String) -> &mut Self {
        self.pre_rendered = Some(content);
        self
    }

    pub fn with_window(&mut self, f: impl FnOnce(WindowBuilder) -> WindowBuilder) -> &mut Self {
        // gots to do a swap because the window builder only takes itself as muy self
        // I wish more people knew about returning &mut Self
        let mut builder = WindowBuilder::default();
        std::mem::swap(&mut self.window, &mut builder);
        builder = f(builder);
        std::mem::swap(&mut self.window, &mut builder);
        self
    }
}
