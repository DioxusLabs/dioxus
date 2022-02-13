use wry::application::window::Icon;
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
    pub(crate) window: WindowBuilder,
    pub(crate) file_drop_handler: Option<Box<dyn Fn(&Window, FileDropEvent) -> bool>>,
    pub(crate) protocols: Vec<WryProtocol>,
    pub(crate) pre_rendered: Option<String>,
    pub(crate) event_handler: Option<Box<DynEventHandlerFn>>,
}

pub type WryProtocol = (
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
            protocols: Vec::new(),
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
        self.protocols.push((name, Box::new(handler)));
        self
    }

    pub fn with_icon(&mut self, icon: Icon) -> &mut Self {
        self.window.window.window_icon = Some(icon);
        self
    }
}

impl DesktopConfig {
    pub(crate) fn with_default_icon(mut self) -> Self {
        let bin: &[u8] = include_bytes!("default_icon.bin");
        let rgba = Icon::from_rgba(bin.to_owned(), 460, 460).expect("image parse failed");
        self.window.window.window_icon = Some(rgba);
        self
    }
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self::new()
    }
}

// dirty trick, avoid introducing `image` at runtime
// TODO: use serde when `Icon` impl serde
//
// This function should only be enabled when generating new icons.
//
// #[test]
// #[ignore]
// fn prepare_default_icon() {
//     use image::io::Reader as ImageReader;
//     use image::ImageFormat;
//     use std::fs::File;
//     use std::io::Cursor;
//     use std::io::Write;
//     use std::path::PathBuf;
//     let png: &[u8] = include_bytes!("default_icon.png");
//     let mut reader = ImageReader::new(Cursor::new(png));
//     reader.set_format(ImageFormat::Png);
//     let icon = reader.decode().unwrap();
//     let bin = PathBuf::from(file!())
//         .parent()
//         .unwrap()
//         .join("default_icon.bin");
//     println!("{:?}", bin);
//     let mut file = File::create(bin).unwrap();
//     file.write_all(icon.as_bytes()).unwrap();
//     println!("({}, {})", icon.width(), icon.height())
// }
