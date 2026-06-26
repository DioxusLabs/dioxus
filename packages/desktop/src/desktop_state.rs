use crate::{
    WindowConfig, assets::AssetHandlerRegistry, edits::EditWebsocket,
    event_handlers::WindowEventHandlers, file_upload::NativeFileHover, ipc::UserWindowEvent,
    query::QueryEngine, shortcut::ShortcutRegistry, webview::PendingWebview,
};
use dioxus_core::{RenderTargetId, Runtime};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
use tao::{
    event_loop::{EventLoopProxy, EventLoopWindowTarget},
    window::{Window, WindowId},
};
use wry::WebView;

/// App-wide desktop state shared by every desktop window.
pub struct DesktopAppContext {
    pub(crate) proxy: EventLoopProxy<UserWindowEvent>,
    pub(crate) target: EventLoopWindowTarget<UserWindowEvent>,
    pub(crate) event_handlers: WindowEventHandlers,
    pub(crate) shortcut_manager: ShortcutRegistry,
    pub(crate) websocket: EditWebsocket,
    pending_webviews: RefCell<Vec<PendingWebview>>,
}

struct ComponentWindowCallbacks {
    on_close_requested: Box<dyn FnMut()>,
    on_destroyed: Box<dyn FnMut()>,
}

/// Result of asking the component tree to close a component-owned window.
pub(crate) enum WindowCloseRequestResult {
    /// A mounted [`Window`](crate::Window) component will remove the portal before native teardown.
    DeferredToComponent,

    /// No mounted component owns this native window, so the app can close it immediately.
    CloseImmediately,
}

impl DesktopAppContext {
    pub(crate) fn new(
        proxy: EventLoopProxy<UserWindowEvent>,
        target: EventLoopWindowTarget<UserWindowEvent>,
    ) -> Self {
        Self {
            proxy,
            target,
            event_handlers: WindowEventHandlers::default(),
            shortcut_manager: ShortcutRegistry::new(),
            websocket: EditWebsocket::start(),
            pending_webviews: RefCell::new(Vec::new()),
        }
    }

    /// Queue a new desktop window from any Dioxus scope.
    pub fn new_window(self: &Rc<Self>, cfg: WindowConfig) -> crate::PendingDesktopWindow {
        let target_id = Runtime::current().create_render_target();
        let (window, context) = PendingWebview::new(target_id, cfg);

        self.proxy.send_event(UserWindowEvent::NewWindow).unwrap();
        self.queue_pending_webview(window);

        context
    }

    /// Shut down the desktop application.
    pub fn shutdown(&self) {
        _ = self.proxy.send_event(UserWindowEvent::Shutdown);
    }

    pub(crate) fn queue_pending_webview(&self, window: PendingWebview) {
        self.pending_webviews.borrow_mut().push(window);
    }

    pub(crate) fn drain_pending_webviews(&self) -> Vec<PendingWebview> {
        self.pending_webviews.borrow_mut().drain(..).collect()
    }
}

/// Native-window state exposed through [`DesktopContext`](crate::DesktopContext).
///
/// Dereferences to the underlying [`tao::window::Window`], so window-manipulation methods such as
/// `set_minimized`, `set_resizable`, or `request_redraw` can be called directly on a
/// [`DesktopContext`](crate::DesktopContext).
pub struct DesktopWindowContext {
    /// The underlying webview handle.
    pub webview: WebView,

    /// The native window handle.
    pub window: Arc<Window>,
    pub(crate) target_id: RenderTargetId,
    pub(crate) asset_handlers: AssetHandlerRegistry,
    pub(crate) file_hover: NativeFileHover,
    pub(crate) query: QueryEngine,
    pub(crate) close_behaviour: Cell<crate::WindowCloseBehaviour>,
    component_window_callbacks: RefCell<Option<ComponentWindowCallbacks>>,

    #[cfg(target_os = "ios")]
    pub(crate) views: RefCell<Vec<objc2::rc::Retained<objc2_ui_kit::UIView>>>,
}

impl DesktopWindowContext {
    pub(crate) fn new(
        webview: WebView,
        window: Arc<Window>,
        target_id: RenderTargetId,
        asset_handlers: AssetHandlerRegistry,
        file_hover: NativeFileHover,
        close_behaviour: crate::WindowCloseBehaviour,
    ) -> Self {
        Self {
            webview,
            window,
            target_id,
            asset_handlers,
            file_hover,
            query: QueryEngine::default(),
            close_behaviour: Cell::new(close_behaviour),
            component_window_callbacks: RefCell::new(None),
            #[cfg(target_os = "ios")]
            views: RefCell::new(Vec::new()),
        }
    }

    /// Get the native window ID.
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Set the native window title.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    pub(crate) fn register_component_window_callbacks(
        &self,
        on_close_requested: impl FnMut() + 'static,
        on_destroyed: impl FnMut() + 'static,
    ) {
        self.component_window_callbacks
            .borrow_mut()
            .replace(ComponentWindowCallbacks {
                on_close_requested: Box::new(on_close_requested),
                on_destroyed: Box::new(on_destroyed),
            });
    }

    pub(crate) fn clear_component_window_callbacks(&self) {
        self.component_window_callbacks.borrow_mut().take();
    }

    pub(crate) fn request_window_close(&self) -> WindowCloseRequestResult {
        if let Some(callbacks) = self.component_window_callbacks.borrow_mut().as_mut() {
            (callbacks.on_close_requested)();
            WindowCloseRequestResult::DeferredToComponent
        } else {
            WindowCloseRequestResult::CloseImmediately
        }
    }

    pub(crate) fn notify_window_destroyed(&self) {
        if let Some(callbacks) = self.component_window_callbacks.borrow_mut().as_mut() {
            (callbacks.on_destroyed)();
        }
    }
}

/// Expose the underlying native window so its [`tao`] methods can be called directly on a
/// [`DesktopContext`](crate::DesktopContext).
impl std::ops::Deref for DesktopWindowContext {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}
