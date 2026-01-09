use crate::{
    app::SharedContext,
    assets::AssetHandlerRegistry,
    file_upload::NativeFileHover,
    ipc::UserWindowEvent,
    query::QueryEngine,
    shortcut::{HotKey, HotKeyState, ShortcutHandle, ShortcutRegistryError},
    webview::PendingWebview,
    AssetRequest, Config, WindowCloseBehaviour, WryEventHandler,
};
use dioxus_core::{Callback, VirtualDom};
use std::{
    cell::Cell,
    future::{Future, IntoFuture},
    pin::Pin,
    rc::{Rc, Weak},
    sync::Arc,
};
use tao::{
    event::Event,
    event_loop::EventLoopWindowTarget,
    window::{Fullscreen as WryFullscreen, Window, WindowId},
};
use wry::{RequestAsyncResponder, WebView};

#[cfg(target_os = "ios")]
use objc2::rc::Retained;
#[cfg(target_os = "ios")]
use objc2_ui_kit::UIView;
#[cfg(target_os = "ios")]
use tao::platform::ios::WindowExtIOS;

/// Get an imperative handle to the current window without using a hook
///
/// ## Panics
///
/// This function will panic if it is called outside of the context of a Dioxus App.
pub fn window() -> DesktopContext {
    dioxus_core::consume_context()
}

/// A handle to the [`DesktopService`] that can be passed around.
pub type DesktopContext = Rc<DesktopService>;

/// A weak handle to the [`DesktopService`] to ensure safe passing.
/// The problem without this is that the tao window is never dropped and therefore cannot be closed.
/// This was due to the Rc that had still references because of multiple copies when creating a webview.
pub type WeakDesktopContext = Weak<DesktopService>;

/// An imperative interface to the current window.
///
/// To get a handle to the current window, use the [`window`] function.
///
///
/// # Example
///
/// you can use `cx.consume_context::<DesktopContext>` to get this context
///
/// ```rust, ignore
///     let desktop = cx.consume_context::<DesktopContext>().unwrap();
/// ```
pub struct DesktopService {
    /// The wry/tao proxy to the current window
    pub webview: WebView,

    /// The tao window itself
    pub window: Arc<Window>,

    pub(crate) shared: Rc<SharedContext>,

    /// The receiver for queries about the current window
    pub(super) query: QueryEngine,
    pub(crate) asset_handlers: AssetHandlerRegistry,
    pub(crate) file_hover: NativeFileHover,
    pub(crate) close_behaviour: Rc<Cell<WindowCloseBehaviour>>,

    #[cfg(target_os = "ios")]
    pub(crate) views: Rc<std::cell::RefCell<Vec<Retained<UIView>>>>,
}

/// A smart pointer to the current window.
impl std::ops::Deref for DesktopService {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DesktopService {
    pub(crate) fn new(
        webview: WebView,
        window: Arc<Window>,
        shared: Rc<SharedContext>,
        asset_handlers: AssetHandlerRegistry,
        file_hover: NativeFileHover,
        close_behaviour: WindowCloseBehaviour,
    ) -> Self {
        Self {
            window,
            webview,
            shared,
            asset_handlers,
            file_hover,
            close_behaviour: Rc::new(Cell::new(close_behaviour)),
            query: Default::default(),
            #[cfg(target_os = "ios")]
            views: Default::default(),
        }
    }

    /// Start the creation of a new window using the props and window builder
    ///
    /// Returns a future that resolves to the webview handle for the new window. You can use this
    /// to control other windows from the current window once the new window is created.
    ///
    /// Be careful to not create a cycle of windows, or you might leak memory.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// fn popup() -> Element {
    ///     rsx! {
    ///         div { "This is a popup window!" }
    ///     }
    /// }
    ///
    /// # async fn app() {
    /// // Create a new window with a component that will be rendered in the new window.
    /// let dom = VirtualDom::new(popup);
    /// // Create and wait for the window
    /// let window = dioxus::desktop::window().new_window(dom, Default::default()).await;
    /// // Fullscreen the new window
    /// window.set_fullscreen(true);
    /// # }
    /// ```
    // Note: This method is asynchronous because webview2 does not support creating a new window from
    // inside of an existing webview callback. Dioxus runs event handlers synchronously inside of a webview
    // callback. See [this page](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/threading-model#reentrancy) for more information.
    //
    // Related issues:
    // - https://github.com/tauri-apps/wry/issues/583
    // - https://github.com/DioxusLabs/dioxus/issues/3080
    pub fn new_window(&self, dom: VirtualDom, cfg: Config) -> PendingDesktopContext {
        let (window, context) = PendingWebview::new(dom, cfg);

        self.shared
            .proxy
            .send_event(UserWindowEvent::NewWindow)
            .unwrap();

        self.shared.pending_webviews.borrow_mut().push(window);

        context
    }

    /// trigger the drag-window event
    ///
    /// Moves the window with the left mouse button until the button is released.
    ///
    /// you need use it in `onmousedown` event:
    /// ```rust, ignore
    /// onmousedown: move |_| { desktop.drag_window(); }
    /// ```
    pub fn drag(&self) {
        if self.window.fullscreen().is_none() {
            _ = self.window.drag_window();
        }
    }

    /// Toggle whether the window is maximized or not
    pub fn toggle_maximized(&self) {
        self.window.set_maximized(!self.window.is_maximized())
    }

    /// Set the close behavior of this window
    ///
    /// By default, windows close when the user clicks the close button.
    /// If this is set to `WindowCloseBehaviour::WindowHides`, the window will hide instead of closing.
    pub fn set_close_behavior(&self, behaviour: WindowCloseBehaviour) {
        self.close_behaviour.set(behaviour);
    }

    /// Close this window
    pub fn close(&self) {
        let _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent::CloseWindow(self.id()));
    }

    /// Close a particular window, given its ID
    pub fn close_window(&self, id: WindowId) {
        let _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent::CloseWindow(id));
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if let Some(handle) = &self.window.current_monitor() {
            self.window.set_fullscreen(
                fullscreen.then_some(WryFullscreen::Borderless(Some(handle.clone()))),
            );
        }
    }

    /// launch print modal
    pub fn print(&self) {
        if let Err(e) = self.webview.print() {
            tracing::warn!("Open print modal failed: {e}");
        }
    }

    /// Set the zoom level of the webview
    pub fn set_zoom_level(&self, level: f64) {
        if let Err(e) = self.webview.zoom(level) {
            tracing::warn!("Set webview zoom failed: {e}");
        }
    }

    /// opens DevTool window
    pub fn devtool(&self) {
        #[cfg(debug_assertions)]
        self.webview.open_devtools();

        #[cfg(not(debug_assertions))]
        tracing::warn!("Devtools are disabled in release builds");
    }

    /// Create a wry event handler that listens for wry events.
    /// This event handler is scoped to the currently active window and will only receive events that are either global or related to the current window.
    ///
    /// The id this function returns can be used to remove the event handler with [`Self::remove_wry_event_handler`]
    pub fn create_wry_event_handler(
        &self,
        handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandler {
        self.shared.event_handlers.add(self.window.id(), handler)
    }

    /// Remove a wry event handler created with [`Self::create_wry_event_handler`]
    pub fn remove_wry_event_handler(&self, id: WryEventHandler) {
        self.shared.event_handlers.remove(id)
    }

    /// Create a global shortcut
    ///
    /// Linux: Only works on x11. See [this issue](https://github.com/tauri-apps/tao/issues/331) for more information.
    pub fn create_shortcut(
        &self,
        hotkey: HotKey,
        callback: impl FnMut(HotKeyState) + 'static,
    ) -> Result<ShortcutHandle, ShortcutRegistryError> {
        self.shared
            .shortcut_manager
            .add_shortcut(hotkey, Box::new(callback))
    }

    /// Remove a global shortcut
    pub fn remove_shortcut(&self, id: ShortcutHandle) {
        self.shared.shortcut_manager.remove_shortcut(id)
    }

    /// Remove all global shortcuts
    pub fn remove_all_shortcuts(&self) {
        self.shared.shortcut_manager.remove_all()
    }

    /// Provide a callback to handle asset loading yourself.
    /// If the ScopeId isn't provided, defaults to a global handler.
    /// Note that the handler is namespaced by name, not ScopeId.
    ///
    /// When the component is dropped, the handler is removed.
    ///
    /// See [`crate::use_asset_handler`] for a convenient hook.
    pub fn register_asset_handler(
        &self,
        name: String,
        handler: impl Fn(AssetRequest, RequestAsyncResponder) + 'static,
    ) {
        self.asset_handlers
            .register_handler(name, Callback::new(move |(req, resp)| handler(req, resp)))
    }

    /// Removes an asset handler by its identifier.
    ///
    /// Returns `None` if the handler did not exist.
    pub fn remove_asset_handler(&self, name: &str) -> Option<()> {
        self.asset_handlers.remove_handler(name).map(|_| ())
    }

    #[cfg(target_os = "ios")]
    /// Get a retained reference to the current UIView
    pub fn ui_view(&self) -> objc2::rc::Retained<objc2_ui_kit::UIView> {
        use objc2::rc::Retained;
        use objc2_ui_kit::UIView;
        let ui_view = self.window.ui_view().cast::<UIView>();
        unsafe { Retained::retain(ui_view) }.unwrap()
    }

    #[cfg(target_os = "ios")]
    /// Get a retained reference to the current UIViewController
    pub fn ui_view_controller(&self) -> objc2::rc::Retained<objc2_ui_kit::UIViewController> {
        use objc2::rc::Retained;
        use objc2_ui_kit::UIViewController;
        let ui_view_controller = self.window.ui_view_controller().cast::<UIViewController>();
        unsafe { Retained::retain(ui_view_controller) }.unwrap()
    }

    /// Push an objc view to the window
    #[cfg(target_os = "ios")]
    pub fn push_view(&self, new_view: Retained<UIView>) {
        use objc2_ui_kit::UIViewAutoresizing;

        assert!(is_main_thread());
        let current_ui_view = self.ui_view();
        let current_ui_view_frame = current_ui_view.frame();

        new_view.setFrame(current_ui_view_frame);
        new_view.setAutoresizingMask(UIViewAutoresizing::from_bits(31).unwrap());

        let ui_view_controller = self.ui_view_controller();
        ui_view_controller.setView(Some(&new_view));
        self.views.borrow_mut().push(new_view);
    }

    /// Pop an objc view from the window
    #[cfg(target_os = "ios")]
    pub fn pop_view(&self) {
        assert!(is_main_thread());
        if let Some(view) = self.views.borrow_mut().pop() {
            self.ui_view_controller().setView(Some(&view));
        }
    }
}

#[cfg(target_os = "ios")]
fn is_main_thread() -> bool {
    objc2_foundation::NSThread::isMainThread_class()
}

/// A [`DesktopContext`] that is pending creation.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # async fn app() {
/// // Create a new window with a component that will be rendered in the new window.
/// let dom = VirtualDom::new(|| rsx!{ "popup!" });
///
/// // Create a new window asynchronously
/// let pending_context = dioxus::desktop::window().new_window(dom, Default::default());
///
/// // Wait for the context to be created
/// let window = pending_context.await;
///
/// // Now control the window
/// window.set_fullscreen(true);
/// # }
/// ```
pub struct PendingDesktopContext {
    pub(crate) receiver: futures_channel::oneshot::Receiver<DesktopContext>,
}

impl PendingDesktopContext {
    /// Resolve the pending context into a [`DesktopContext`].
    pub async fn resolve(self) -> DesktopContext {
        self.try_resolve()
            .await
            .expect("Failed to resolve pending desktop context")
    }

    /// Try to resolve the pending context into a [`DesktopContext`].
    pub async fn try_resolve(self) -> Result<DesktopContext, futures_channel::oneshot::Canceled> {
        self.receiver.await
    }
}

impl IntoFuture for PendingDesktopContext {
    type Output = DesktopContext;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.resolve())
    }
}
