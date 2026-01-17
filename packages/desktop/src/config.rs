use dioxus_core::{LaunchConfig, VirtualDom};
use std::path::PathBuf;
use std::{borrow::Cow, sync::Arc};
use tao::window::{Icon, WindowBuilder};
use tao::{
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::Window,
};
use wry::http::{Request as HttpRequest, Response as HttpResponse};
use wry::{RequestAsyncResponder, WebViewId};

use crate::ipc::UserWindowEvent;
use crate::menubar::{default_menu_bar, DioxusMenu};

type CustomEventHandler = Box<
    dyn 'static
        + for<'a> FnMut(
            &tao::event::Event<'a, UserWindowEvent>,
            &EventLoopWindowTarget<UserWindowEvent>,
        ),
>;

/// The closing behaviour of specific application window.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowCloseBehaviour {
    /// Window will hide instead of closing
    WindowHides,

    /// Window will close
    WindowCloses,
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
    pub(crate) event_loop: Option<EventLoop<UserWindowEvent>>,
    pub(crate) window: WindowBuilder,
    pub(crate) as_child_window: bool,
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
    pub(crate) exit_on_last_window_close: bool,
    pub(crate) window_close_behavior: WindowCloseBehaviour,
    pub(crate) custom_event_handler: Option<CustomEventHandler>,
    pub(crate) disable_file_drop_handler: bool,
    pub(crate) disable_dma_buf_on_wayland: bool,
    pub(crate) additional_windows_args: Option<String>,

    #[allow(clippy::type_complexity)]
    pub(crate) on_window: Option<Box<dyn FnMut(Arc<Window>, &mut VirtualDom) + 'static>>,
}

impl LaunchConfig for Config {}

pub(crate) type WryProtocol = (
    String,
    Box<dyn Fn(WebViewId, HttpRequest<Vec<u8>>) -> HttpResponse<Cow<'static, [u8]>> + 'static>,
);

pub(crate) type AsyncWryProtocol = (
    String,
    Box<dyn Fn(WebViewId, HttpRequest<Vec<u8>>, RequestAsyncResponder) + 'static>,
);

impl Config {
    /// Initializes a new `WindowBuilder` with default values.
    #[inline]
    pub fn new() -> Self {
        let mut window: WindowBuilder = WindowBuilder::new()
            .with_title(dioxus_cli_config::app_title().unwrap_or_else(|| "Dioxus App".to_string()));

        // During development we want the window to be on top so we can see it while we work
        let always_on_top = dioxus_cli_config::always_on_top().unwrap_or(true);

        if cfg!(debug_assertions) {
            window = window.with_always_on_top(always_on_top);
        }

        Self {
            window,
            as_child_window: false,
            event_loop: None,
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
            exit_on_last_window_close: true,
            window_close_behavior: WindowCloseBehaviour::WindowCloses,
            custom_event_handler: None,
            disable_file_drop_handler: false,
            disable_dma_buf_on_wayland: true,
            on_window: None,
            additional_windows_args: None,
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

    /// Set whether or not the file drop handler should be disabled.
    /// On Windows the drop handler must be disabled for HTML drag and drop APIs to work.
    pub fn with_disable_drag_drop_handler(mut self, disable: bool) -> Self {
        self.disable_file_drop_handler = disable;
        self
    }

    /// Set the pre-rendered HTML content
    pub fn with_prerendered(mut self, content: String) -> Self {
        self.pre_rendered = Some(content);
        self
    }

    /// Set the event loop to be used
    pub fn with_event_loop(mut self, event_loop: EventLoop<UserWindowEvent>) -> Self {
        self.event_loop = Some(event_loop);
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

    /// Set the window as child
    pub fn with_as_child_window(mut self) -> Self {
        self.as_child_window = true;
        self
    }

    /// When the last window is closed, the application will exit.
    ///
    /// This is the default behaviour.
    ///
    /// If the last window is hidden, the application will not exit.
    pub fn with_exits_when_last_window_closes(mut self, exit: bool) -> Self {
        self.exit_on_last_window_close = exit;
        self
    }

    /// Sets the behaviour of the application when the last window is closed.
    pub fn with_close_behaviour(mut self, behaviour: WindowCloseBehaviour) -> Self {
        self.window_close_behavior = behaviour;
        self
    }

    /// Sets a custom callback to run whenever the event pool receives an event.
    pub fn with_custom_event_handler(
        mut self,
        f: impl FnMut(&tao::event::Event<'_, UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>)
            + 'static,
    ) -> Self {
        self.custom_event_handler = Some(Box::new(f));
        self
    }

    /// Set a custom protocol
    pub fn with_custom_protocol<F>(mut self, name: impl ToString, handler: F) -> Self
    where
        F: Fn(WebViewId, HttpRequest<Vec<u8>>) -> HttpResponse<Cow<'static, [u8]>> + 'static,
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
    ///     .with_asynchronous_custom_protocol("asset", |_webview_id, request, responder| {
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
        F: Fn(WebViewId, HttpRequest<Vec<u8>>, RequestAsyncResponder) + 'static,
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

    /// Allows modifying the window and virtual dom right after they are built, but before the webview is created.
    ///
    /// This is important for z-ordering textures in child windows. Note that this callback runs on
    /// every window creation, so it's up to you to
    pub fn with_on_window(mut self, f: impl FnMut(Arc<Window>, &mut VirtualDom) + 'static) -> Self {
        self.on_window = Some(Box::new(f));
        self
    }

    /// Set whether or not DMA-BUF usage should be disabled on Wayland.
    ///
    /// Defaults to true to avoid issues on some systems. If you want to enable DMA-BUF usage, set this to false.
    /// See <https://github.com/DioxusLabs/dioxus/issues/4528#issuecomment-3476430611>
    pub fn with_disable_dma_buf_on_wayland(mut self, disable: bool) -> Self {
        self.disable_dma_buf_on_wayland = disable;
        self
    }

    /// Add additional windows only launch arguments for webview2
    pub fn with_windows_browser_args(mut self, additional_args: impl ToString) -> Self {
        self.additional_windows_args = Some(additional_args.to_string());
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
