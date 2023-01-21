use std::path::PathBuf;

use wry::application::window::Icon;
use wry::{
    application::window::{Window, WindowBuilder},
    http::{Request as HttpRequest, Response as HttpResponse},
    webview::FileDropEvent,
    Result as WryResult,
};

// pub(crate) type DynEventHandlerFn = dyn Fn(&mut EventLoop<()>, &mut WebView);

/// The configuration for the desktop application.
pub struct Config {
    pub(crate) window: WindowBuilder,
    pub(crate) file_drop_handler: Option<DropHandler>,
    pub(crate) protocols: Vec<WryProtocol>,
    pub(crate) pre_rendered: Option<String>,
    pub(crate) disable_context_menu: bool,
    pub(crate) resource_dir: Option<PathBuf>,
    pub(crate) custom_head: Option<String>,
    pub(crate) custom_index: Option<String>,
    pub(crate) root_name: String,
}

type DropHandler = Box<dyn Fn(&Window, FileDropEvent) -> bool>;

pub(crate) type WryProtocol = (
    String,
    Box<dyn Fn(&HttpRequest<Vec<u8>>) -> WryResult<HttpResponse<Vec<u8>>> + 'static>,
);

impl Config {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        let window = WindowBuilder::new().with_title("Dioxus app");

        Self {
            // event_handler: None,
            window,
            protocols: Vec::new(),
            file_drop_handler: None,
            pre_rendered: None,
            disable_context_menu: !cfg!(debug_assertions),
            resource_dir: None,
            custom_head: None,
            custom_index: None,
            root_name: "main".to_string(),
        }
    }

    /// set the directory from which assets will be searched in release mode
    pub fn with_resource_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.resource_dir = Some(path.into());
        self
    }

    /// Set whether or not the right-click context menu should be disabled.
    pub fn with_disable_context_menu(mut self, disable: bool) -> Self {
        self.disable_context_menu = disable;
        self
    }

    /// Set the pre-rendered HTML content
    pub fn with_prerendered(mut self, content: String) -> Self {
        self.pre_rendered = Some(content);
        self
    }

    /// Set the configuration for the window.
    pub fn with_window(mut self, window: WindowBuilder) -> Self {
        // gots to do a swap because the window builder only takes itself as muy self
        // I wish more people knew about returning &mut Self
        self.window = window;
        self
    }

    /// Set a file drop handler
    pub fn with_file_drop_handler(
        mut self,
        handler: impl Fn(&Window, FileDropEvent) -> bool + 'static,
    ) -> Self {
        self.file_drop_handler = Some(Box::new(handler));
        self
    }

    /// Set a custom protocol
    pub fn with_custom_protocol<F>(mut self, name: String, handler: F) -> Self
    where
        F: Fn(&HttpRequest<Vec<u8>>) -> WryResult<HttpResponse<Vec<u8>>> + 'static,
    {
        self.protocols.push((name, Box::new(handler)));
        self
    }

    /// Set a custom icon for this application
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.window.window.window_icon = Some(icon);
        self
    }

    /// Inject additional content into the document's HEAD.
    ///
    /// This is useful for loading CSS libraries, JS libraries, etc.
    pub fn with_custom_head(mut self, head: String) -> Self {
        self.custom_head = Some(head);
        self
    }

    /// Use a custom index.html instead of the default Dioxus one.
    ///
    /// Make sure your index.html is valid HTML.
    ///
    /// Dioxus injects some loader code into the closing body tag. Your document
    /// must include a body element!
    pub fn with_custom_index(mut self, index: String) -> Self {
        self.custom_index = Some(index);
        self
    }

    /// Set the name of the element that Dioxus will use as the root.
    ///
    /// This is akint to calling React.render() on the element with the specified name.
    pub fn with_root_name(mut self, name: impl Into<String>) -> Self {
        self.root_name = name.into();
        self
    }
}

impl Default for Config {
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
