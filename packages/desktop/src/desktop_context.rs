use crate::{
    app::SharedContext,
    assets::AssetHandlerRegistry,
    edits::EditQueue,
    ipc::{EventData, UserWindowEvent},
    query::QueryEngine,
    shortcut::{HotKey, ShortcutId, ShortcutRegistryError},
    webview::WebviewInstance,
    AssetRequest, Config,
};
use dioxus_core::{
    prelude::{current_scope_id, ScopeId},
    Mutations, VirtualDom,
};
use dioxus_interpreter_js::binary_protocol::Channel;
use rustc_hash::FxHashMap;
use slab::Slab;
use std::{cell::RefCell, fmt::Debug, rc::Rc, rc::Weak, sync::atomic::AtomicU16};
use tao::{
    event::Event,
    event_loop::EventLoopWindowTarget,
    window::{Fullscreen as WryFullscreen, Window, WindowId},
};
use wry::{RequestAsyncResponder, WebView};

#[cfg(target_os = "ios")]
use tao::platform::ios::WindowExtIOS;

/// Get an imperative handle to the current window without using a hook
///
/// ## Panics
///
/// This function will panic if it is called outside of the context of a Dioxus App.
pub fn window() -> DesktopContext {
    dioxus_core::prelude::consume_context().unwrap()
}

/// A handle to the [`DesktopService`] that can be passed around.
pub type DesktopContext = Rc<DesktopService>;

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
    pub webview: WebView,

    /// The tao window itself
    pub window: Window,

    pub(crate) shared: Rc<SharedContext>,

    /// The receiver for queries about the current window
    pub(super) query: QueryEngine,
    pub(crate) edit_queue: EditQueue,
    pub(crate) templates: RefCell<FxHashMap<String, u16>>,
    pub(crate) max_template_count: AtomicU16,
    pub(crate) channel: RefCell<Channel>,
    pub(crate) asset_handlers: AssetHandlerRegistry,

    #[cfg(target_os = "ios")]
    pub(crate) views: Rc<RefCell<Vec<*mut objc::runtime::Object>>>,
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
        window: Window,
        shared: Rc<SharedContext>,
        edit_queue: EditQueue,
        asset_handlers: AssetHandlerRegistry,
    ) -> Self {
        Self {
            window,
            webview,
            shared,
            edit_queue,
            asset_handlers,
            query: Default::default(),
            templates: Default::default(),
            max_template_count: Default::default(),
            channel: Default::default(),
            #[cfg(target_os = "ios")]
            views: Default::default(),
        }
    }

    /// Send a list of mutations to the webview
    pub(crate) fn send_edits(&self, edits: Mutations) {
        if let Some(bytes) = crate::edits::apply_edits(
            edits,
            &mut self.channel.borrow_mut(),
            &mut self.templates.borrow_mut(),
            &self.max_template_count,
        ) {
            self.edit_queue.add_edits(bytes)
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
        let window = WebviewInstance::new(cfg, dom, self.shared.clone());

        let cx = window.desktop_context.clone();

        self.shared
            .proxy
            .send_event(UserWindowEvent(EventData::NewWindow, cx.id()))
            .unwrap();

        self.shared
            .proxy
            .send_event(UserWindowEvent(EventData::Poll, cx.id()))
            .unwrap();

        self.shared.pending_webviews.borrow_mut().push(window);

        Rc::downgrade(&cx)
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

    /// Close this window
    pub fn close(&self) {
        let _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent(EventData::CloseWindow, self.id()));
    }

    /// Close a particular window, given its ID
    pub fn close_window(&self, id: WindowId) {
        let _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent(EventData::CloseWindow, id));
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
        self.shared.event_handlers.add(self.window.id(), handler)
    }

    /// Remove a wry event handler created with [`DesktopContext::create_wry_event_handler`]
    pub fn remove_wry_event_handler(&self, id: WryEventHandlerId) {
        self.shared.event_handlers.remove(id)
    }

    /// Create a global shortcut
    ///
    /// Linux: Only works on x11. See [this issue](https://github.com/tauri-apps/tao/issues/331) for more information.
    pub fn create_shortcut(
        &self,
        hotkey: HotKey,
        callback: impl FnMut() + 'static,
    ) -> Result<ShortcutId, ShortcutRegistryError> {
        self.shared
            .shortcut_manager
            .add_shortcut(hotkey, Box::new(callback))
    }

    /// Remove a global shortcut
    pub fn remove_shortcut(&self, id: ShortcutId) {
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
    /// See [`use_asset_handle`](crate::use_asset_handle) for a convenient hook.
    pub fn register_asset_handler(
        &self,
        name: String,
        f: Box<dyn Fn(AssetRequest, RequestAsyncResponder) + 'static>,
        scope: Option<ScopeId>,
    ) {
        self.asset_handlers.register_handler(
            name,
            f,
            scope.unwrap_or(current_scope_id().unwrap_or(ScopeId(0))),
        )
    }

    /// Removes an asset handler by its identifier.
    ///
    /// Returns `None` if the handler did not exist.
    pub fn remove_asset_handler(&self, name: &str) -> Option<()> {
        self.asset_handlers.remove_handler(name).map(|_| ())
    }

    /// Push an objc view to the window
    #[cfg(target_os = "ios")]
    pub fn push_view(&self, view: objc_id::ShareId<objc::runtime::Object>) {
        let window = &self.window;

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
        let window = &self.window;

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

/// A wry event handler that is scoped to the current component and window. The event handler will only receive events for the window it was created for and global events.
///
/// This will automatically be removed when the component is unmounted.
pub struct WryEventHandler {
    pub(crate) handlers: WindowEventHandlers,
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
