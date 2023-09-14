use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

use crate::create_new_window;
use crate::events::IpcMessage;
use crate::query::QueryEngine;
use crate::shortcut::{HotKey, ShortcutId, ShortcutRegistry, ShortcutRegistryError};
use crate::Config;
use crate::WebviewHandler;
use dioxus_core::ScopeState;
use dioxus_core::VirtualDom;
#[cfg(all(feature = "hot-reload", debug_assertions))]
use dioxus_hot_reload::HotReloadMsg;
use slab::Slab;
use wry::application::event::Event;
use wry::application::event_loop::EventLoopProxy;
use wry::application::event_loop::EventLoopWindowTarget;
#[cfg(target_os = "ios")]
use wry::application::platform::ios::WindowExtIOS;
use wry::application::window::Fullscreen as WryFullscreen;
use wry::application::window::Window;
use wry::application::window::WindowId;
use wry::webview::WebView;

pub type ProxyType = EventLoopProxy<UserWindowEvent>;

/// Get an imperative handle to the current window
pub fn use_window(cx: &ScopeState) -> &DesktopContext {
    cx.use_hook(|| cx.consume_context::<DesktopContext>())
        .as_ref()
        .unwrap()
}

pub(crate) type WebviewQueue = Rc<RefCell<Vec<WebviewHandler>>>;

/// An imperative interface to the current window.
///
/// To get a handle to the current window, use the [`use_window`] hook.
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
    pub webview: Rc<WebView>,

    /// The proxy to the event loop
    pub proxy: ProxyType,

    /// The receiver for queries about the current window
    pub(super) query: QueryEngine,

    pub(super) pending_windows: WebviewQueue,

    pub(crate) event_loop: EventLoopWindowTarget<UserWindowEvent>,

    pub(crate) event_handlers: WindowEventHandlers,

    pub(crate) shortcut_manager: ShortcutRegistry,

    #[cfg(target_os = "ios")]
    pub(crate) views: Rc<RefCell<Vec<*mut objc::runtime::Object>>>,
}

/// A handle to the [`DesktopService`] that can be passed around.
pub type DesktopContext = Rc<DesktopService>;

/// A smart pointer to the current window.
impl std::ops::Deref for DesktopService {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        self.webview.window()
    }
}

impl DesktopService {
    pub(crate) fn new(
        webview: WebView,
        proxy: ProxyType,
        event_loop: EventLoopWindowTarget<UserWindowEvent>,
        webviews: WebviewQueue,
        event_handlers: WindowEventHandlers,
        shortcut_manager: ShortcutRegistry,
    ) -> Self {
        Self {
            webview: Rc::new(webview),
            proxy,
            event_loop,
            query: Default::default(),
            pending_windows: webviews,
            event_handlers,
            shortcut_manager,
            #[cfg(target_os = "ios")]
            views: Default::default(),
        }
    }

    /// Create a new window using the props and window builder
    ///
    /// Returns the webview handle for the new window.
    ///
    /// You can use this to control other windows from the current window.
    ///
    /// Be careful to not create a cycle of windows, or you might leak memory.
    pub fn new_window(&self, dom: VirtualDom, cfg: Config) -> Weak<DesktopService> {
        let window = create_new_window(
            cfg,
            &self.event_loop,
            &self.proxy,
            dom,
            &self.pending_windows,
            &self.event_handlers,
            self.shortcut_manager.clone(),
        );

        let desktop_context = window
            .dom
            .base_scope()
            .consume_context::<Rc<DesktopService>>()
            .unwrap();

        let id = window.desktop_context.webview.window().id();

        self.proxy
            .send_event(UserWindowEvent(EventData::NewWindow, id))
            .unwrap();

        self.proxy
            .send_event(UserWindowEvent(EventData::Poll, id))
            .unwrap();

        self.pending_windows.borrow_mut().push(window);

        Rc::downgrade(&desktop_context)
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
        let window = self.webview.window();

        // if the drag_window has any errors, we don't do anything
        if window.fullscreen().is_none() {
            window.drag_window().unwrap();
        }
    }

    /// Toggle whether the window is maximized or not
    pub fn toggle_maximized(&self) {
        let window = self.webview.window();

        window.set_maximized(!window.is_maximized())
    }

    /// close window
    pub fn close(&self) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent(EventData::CloseWindow, self.id()));
    }

    /// close window
    pub fn close_window(&self, id: WindowId) {
        let _ = self
            .proxy
            .send_event(UserWindowEvent(EventData::CloseWindow, id));
    }

    /// change window to fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if let Some(handle) = self.webview.window().current_monitor() {
            self.webview
                .window()
                .set_fullscreen(fullscreen.then_some(WryFullscreen::Borderless(Some(handle))));
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
        self.webview.zoom(level);
    }

    /// opens DevTool window
    pub fn devtool(&self) {
        #[cfg(debug_assertions)]
        self.webview.open_devtools();

        #[cfg(not(debug_assertions))]
        tracing::warn!("Devtools are disabled in release builds");
    }

    /// Create a wry event handler that listens for wry events.
    /// This event handler is scoped to the currently active window and will only recieve events that are either global or related to the current window.
    ///
    /// The id this function returns can be used to remove the event handler with [`DesktopContext::remove_wry_event_handler`]
    pub fn create_wry_event_handler(
        &self,
        handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandlerId {
        self.event_handlers.add(self.id(), handler)
    }

    /// Remove a wry event handler created with [`DesktopContext::create_wry_event_handler`]
    pub fn remove_wry_event_handler(&self, id: WryEventHandlerId) {
        self.event_handlers.remove(id)
    }

    /// Create a global shortcut
    ///
    /// Linux: Only works on x11. See [this issue](https://github.com/tauri-apps/tao/issues/331) for more information.
    pub fn create_shortcut(
        &self,
        hotkey: HotKey,
        callback: impl FnMut() + 'static,
    ) -> Result<ShortcutId, ShortcutRegistryError> {
        self.shortcut_manager
            .add_shortcut(hotkey, Box::new(callback))
    }

    /// Remove a global shortcut
    pub fn remove_shortcut(&self, id: ShortcutId) {
        self.shortcut_manager.remove_shortcut(id)
    }

    /// Remove all global shortcuts
    pub fn remove_all_shortcuts(&self) {
        self.shortcut_manager.remove_all()
    }

    /// Push an objc view to the window
    #[cfg(target_os = "ios")]
    pub fn push_view(&self, view: objc_id::ShareId<objc::runtime::Object>) {
        let window = self.webview.window();

        unsafe {
            use objc::runtime::Object;
            use objc::*;
            assert!(is_main_thread());
            let ui_view = window.ui_view() as *mut Object;
            let ui_view_frame: *mut Object = msg_send![ui_view, frame];
            let _: () = msg_send![view, setFrame: ui_view_frame];
            let _: () = msg_send![view, setAutoresizingMask: 31];

            let ui_view_controller = window.ui_view_controller() as *mut Object;
            let _: () = msg_send![ui_view_controller, setView: view];
            self.views.borrow_mut().push(ui_view);
        }
    }

    /// Pop an objc view from the window
    #[cfg(target_os = "ios")]
    pub fn pop_view(&self) {
        let window = self.webview.window();

        unsafe {
            use objc::runtime::Object;
            use objc::*;
            assert!(is_main_thread());
            if let Some(view) = self.views.borrow_mut().pop() {
                let ui_view_controller = window.ui_view_controller() as *mut Object;
                let _: () = msg_send![ui_view_controller, setView: view];
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserWindowEvent(pub EventData, pub WindowId);

#[derive(Debug, Clone)]
pub enum EventData {
    Poll,

    Ipc(IpcMessage),

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    HotReloadEvent(HotReloadMsg),

    NewWindow,

    CloseWindow,
}

#[cfg(target_os = "ios")]
fn is_main_thread() -> bool {
    use objc::runtime::{Class, BOOL, NO};
    use objc::*;

    let cls = Class::get("NSThread").unwrap();
    let result: BOOL = unsafe { msg_send![cls, isMainThread] };
    result != NO
}

/// The unique identifier of a window event handler. This can be used to later remove the handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WryEventHandlerId(usize);

#[derive(Clone, Default)]
pub(crate) struct WindowEventHandlers {
    handlers: Rc<RefCell<Slab<WryWindowEventHandlerInner>>>,
}

impl WindowEventHandlers {
    pub(crate) fn add(
        &self,
        window_id: WindowId,
        handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandlerId {
        WryEventHandlerId(
            self.handlers
                .borrow_mut()
                .insert(WryWindowEventHandlerInner {
                    window_id,
                    handler: Box::new(handler),
                }),
        )
    }

    pub(crate) fn remove(&self, id: WryEventHandlerId) {
        self.handlers.borrow_mut().try_remove(id.0);
    }

    pub(crate) fn apply_event(
        &self,
        event: &Event<UserWindowEvent>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) {
        for (_, handler) in self.handlers.borrow_mut().iter_mut() {
            handler.apply_event(event, target);
        }
    }
}

struct WryWindowEventHandlerInner {
    window_id: WindowId,
    handler: WryEventHandlerCallback,
}

type WryEventHandlerCallback =
    Box<dyn FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static>;

impl WryWindowEventHandlerInner {
    fn apply_event(
        &mut self,
        event: &Event<UserWindowEvent>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) {
        // if this event does not apply to the window this listener cares about, return
        if let Event::WindowEvent { window_id, .. } = event {
            if *window_id != self.window_id {
                return;
            }
        }
        (self.handler)(event, target)
    }
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_wry_event_handler(
    cx: &ScopeState,
    handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
) -> &WryEventHandler {
    let desktop = use_window(cx);
    cx.use_hook(move || {
        let desktop = desktop.clone();

        let id = desktop.create_wry_event_handler(handler);

        WryEventHandler {
            handlers: desktop.event_handlers.clone(),
            id,
        }
    })
}

/// A wry event handler that is scoped to the current component and window. The event handler will only receive events for the window it was created for and global events.
///
/// This will automatically be removed when the component is unmounted.
pub struct WryEventHandler {
    handlers: WindowEventHandlers,
    /// The unique identifier of the event handler.
    pub id: WryEventHandlerId,
}

impl WryEventHandler {
    /// Remove the event handler.
    pub fn remove(&self) {
        self.handlers.remove(self.id);
    }
}

impl Drop for WryEventHandler {
    fn drop(&mut self) {
        self.handlers.remove(self.id);
    }
}
