use std::borrow::Cow;
use std::path::PathBuf;
use tao::window::{Icon, WindowBuilder};
use wry::http::{Request as HttpRequest, Response as HttpResponse};
use wry::RequestAsyncResponder;

use crate::menubar::{default_menu_bar, DioxusMenu};

/// The behaviour of the application when the last window is closed.
#[derive(Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowCloseBehaviour {
    /// Default behaviour, closing the last window exits the app
    LastWindowExitsApp,
    /// Closing the last window will not actually close it, just hide it
    LastWindowHides,
    /// Closing the last window will close it but the app will keep running so that new windows can be opened
    CloseWindow,
}

/// The state of the menu builder. We need to keep track of if the state is default
/// so we only swap out the default menu bar when decorations are disabled
pub(crate) enum MenuBuilderState {
    Unset,
    Set(Option<DioxusMenu>),
}

impl From<MenuBuilderState> for Option<DioxusMenu> {
    fn from(val: MenuBuilderState) -> Self {
        match val {
            MenuBuilderState::Unset => Some(default_menu_bar()),
            MenuBuilderState::Set(menu) => menu,
        }
    }
}

/// The configuration for the desktop application.
pub struct Config {
    pub(crate) window: WindowBuilder,
    pub(crate) menu: MenuBuilderState,
    pub(crate) protocols: Vec<WryProtocol>,
    pub(crate) asynchronous_protocols: Vec<AsyncWryProtocol>,
    pub(crate) pre_rendered: Option<String>,
    pub(crate) disable_context_menu: bool,
    pub(crate) resource_dir: Option<PathBuf>,
    pub(crate) data_dir: Option<PathBuf>,
    pub(crate) custom_head: Option<String>,
    pub(crate) custom_index: Option<String>,
    pub(crate) root_name: String,
    pub(crate) background_color: Option<(u8, u8, u8, u8)>,
    pub(crate) last_window_close_behavior: WindowCloseBehaviour,
}

pub(crate) type WryProtocol = (
    String,
    Box<dyn Fn(HttpRequest<Vec<u8>>) -> HttpResponse<Cow<'static, [u8]>> + 'static>,
);

pub(crate) type AsyncWryProtocol = (
    String,
    Box<dyn Fn(HttpRequest<Vec<u8>>, RequestAsyncResponder) + 'static>,
);

impl Config {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        // let dioxus_config = dioxus_cli_config::CURRENT_CONFIG.as_ref();

        let mut window: WindowBuilder = WindowBuilder::new().with_title(
            "Dioxus App".to_string(),
            // dioxus_config
            //     .map(|c| c.application.name.clone())
            //     .unwrap_or("Dioxus App".to_string()),
        );

        // During development we want the window to be on top so we can see it while we work
        // let always_on_top = dioxus_config
        //     // .map(|c| c.desktop.always_on_top)
        //     .unwrap_or(true);
        let always_on_top = true;

        if cfg!(debug_assertions) {
            window = window.with_always_on_top(always_on_top);
        }

        Self {
            window,
            menu: MenuBuilderState::Unset,
            protocols: Vec::new(),
            asynchronous_protocols: Vec::new(),
            pre_rendered: None,
            disable_context_menu: !cfg!(debug_assertions),
            resource_dir: None,
            data_dir: None,
            custom_head: None,
            custom_index: None,
            root_name: "main".to_string(),
            background_color: None,
            last_window_close_behavior: WindowCloseBehaviour::LastWindowExitsApp,
        }
    }

    /// set the directory from which assets will be searched in release mode
    pub fn with_resource_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.resource_dir = Some(path.into());
        self
    }

    /// set the directory where data will be stored in release mode.
    ///
    /// > Note: This **must** be set when bundling on Windows.
    pub fn with_data_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.data_dir = Some(path.into());
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
        // We need to do a swap because the window builder only takes itself as muy self
        self.window = window;
        // If the decorations are off for the window, remove the menu as well
        if !self.window.window.decorations && matches!(self.menu, MenuBuilderState::Unset) {
            self.menu = MenuBuilderState::Set(None);
        }
        self
    }

    /// Sets the behaviour of the application when the last window is closed.
    pub fn with_close_behaviour(mut self, behaviour: WindowCloseBehaviour) -> Self {
        self.last_window_close_behavior = behaviour;
        self
    }

    /// Set a custom protocol
    pub fn with_custom_protocol<F>(mut self, name: impl ToString, handler: F) -> Self
    where
        F: Fn(HttpRequest<Vec<u8>>) -> HttpResponse<Cow<'static, [u8]>> + 'static,
    {
        self.protocols.push((name.to_string(), Box::new(handler)));
        self
    }

    /// Set an asynchronous custom protocol
    ///
    /// **Example Usage**
    /// ```rust
    /// # use wry::http::response::Response as HTTPResponse;
    /// # use std::borrow::Cow;
    /// # use dioxus_desktop::Config;
    /// #
    /// # fn main() {
    /// let cfg = Config::new()
    ///     .with_asynchronous_custom_protocol("asset", |request, responder| {
    ///         tokio::spawn(async move {
    ///             responder.respond(
    ///                 HTTPResponse::builder()
    ///                     .status(404)
    ///                     .body(Cow::Borrowed("404 - Not Found".as_bytes()))
    ///                     .unwrap()
    ///             );
    ///         });
    ///     });
    /// # }
    /// ```
    /// note a key difference between Dioxus and Wry, the protocol name doesn't explicitly need to be a
    /// [`String`], but needs to implement [`ToString`].
    ///
    /// See [`wry`](wry::WebViewBuilder::with_asynchronous_custom_protocol) for more details on implementation
    pub fn with_asynchronous_custom_protocol<F>(mut self, name: impl ToString, handler: F) -> Self
    where
        F: Fn(HttpRequest<Vec<u8>>, RequestAsyncResponder) + 'static,
    {
        self.asynchronous_protocols
            .push((name.to_string(), Box::new(handler)));
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
    /// This is akin to calling React.render() on the element with the specified name.
    pub fn with_root_name(mut self, name: impl Into<String>) -> Self {
        self.root_name = name.into();
        self
    }

    /// Sets the background color of the WebView.
    /// This will be set before the HTML is rendered and can be used to prevent flashing when the page loads.
    /// Accepts a color in RGBA format
    pub fn with_background_color(mut self, color: (u8, u8, u8, u8)) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Sets the menu the window will use. This will override the default menu bar.
    ///
    /// > Note: Menu will be hidden if
    /// > [`with_decorations`](tao::window::WindowBuilder::with_decorations)
    /// > is set to false and passed into [`with_window`](Config::with_window)
    #[allow(unused)]
    pub fn with_menu(mut self, menu: impl Into<Option<DioxusMenu>>) -> Self {
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            if self.window.window.decorations {
                self.menu = MenuBuilderState::Set(menu.into())
            }
        }
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
