use bevy::utils::Duration;
use std::{
    fmt::{self, Debug},
    path::PathBuf,
};

use dioxus_desktop::wry::{
    application::{event_loop::EventLoop, window::Window},
    http::{Request as HttpRequest, Response as HttpResponse},
    webview::{FileDropEvent, WebView},
    Result as WryResult,
};

pub struct DioxusDesktopSettings {
    pub focused_mode: UpdateMode,
    pub unfocused_mode: UpdateMode,

    pub file_drop_handler: Option<Box<dyn Fn(&Window, FileDropEvent) -> bool>>,
    pub protocols: Vec<WryProtocol>,
    pub pre_rendered: Option<String>,
    pub event_handler: Option<Box<DynEventHandlerFn>>,
    pub disable_context_menu: bool,
    pub resource_dir: Option<PathBuf>,
    pub custom_head: Option<String>,
    pub custom_index: Option<String>,
}

pub type WryProtocol = (
    String,
    Box<dyn Fn(&HttpRequest) -> WryResult<HttpResponse> + 'static>,
);

pub type DynEventHandlerFn = dyn Fn(&mut EventLoop<()>, &mut WebView);

impl Debug for DioxusDesktopSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DioxusWindows")
            .field("focused_mode", &self.focused_mode)
            .field("unfocused_mode", &self.unfocused_mode)
            .finish()
    }
}

impl DioxusDesktopSettings {
    pub fn game() -> Self {
        DioxusDesktopSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
            ..Default::default()
        }
    }

    pub fn update_mode(&self, focused: bool) -> &UpdateMode {
        match focused {
            true => &self.focused_mode,
            false => &self.unfocused_mode,
        }
    }

    pub fn with_resource_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.resource_dir = Some(path.into());
        self
    }

    pub fn with_disable_context_menu(&mut self, disable: bool) -> &mut Self {
        self.disable_context_menu = disable;
        self
    }

    pub fn with_prerendered(&mut self, content: String) -> &mut Self {
        self.pre_rendered = Some(content);
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
        self.protocols.push((name, Box::new(handler)));
        self
    }

    pub fn with_custom_head(&mut self, head: String) -> &mut Self {
        self.custom_head = Some(head);
        self
    }

    pub fn with_custom_index(&mut self, index: String) -> &mut Self {
        self.custom_index = Some(index);
        self
    }
}

impl Default for DioxusDesktopSettings {
    fn default() -> Self {
        DioxusDesktopSettings {
            focused_mode: UpdateMode::Reactive {
                max_wait: Duration::from_secs(5),
            },
            unfocused_mode: UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_secs(60),
            },

            protocols: Vec::new(),
            file_drop_handler: None,
            pre_rendered: None,
            event_handler: None,
            disable_context_menu: !cfg!(debug_assertions),
            resource_dir: None,
            custom_head: None,
            custom_index: None,
        }
    }
}

#[derive(Debug)]
pub enum UpdateMode {
    Continuous,
    Reactive { max_wait: Duration },
    ReactiveLowPower { max_wait: Duration },
}
