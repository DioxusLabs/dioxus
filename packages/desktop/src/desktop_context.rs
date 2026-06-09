use crate::{
    AssetRequest, Config, WindowCloseBehaviour, WryEventHandler,
    app::SharedContext,
    assets::AssetHandlerRegistry,
    dom_thread::{
        DomCallbackId, PendingDomId, SharedCallbackRegistry, VirtualDomEvent, reserve_pending_dom,
    },
    ipc::{DesktopServiceCallback, UserWindowEvent},
    shortcut::{HotKey, HotKeyState, ShortcutHandle, ShortcutRegistryError},
    webview::PendingWebview,
};
use dioxus_core::{Callback, VirtualDom};
use std::{
    cell::Cell,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    sync::Arc,
};
use tao::{
    dpi::{PhysicalPosition, PhysicalSize, Position, Size},
    error::{ExternalError, NotSupportedError},
    event::{Event, WindowEvent},
    event_loop::{EventLoopProxy, EventLoopWindowTarget},
    monitor::MonitorHandle,
    window::{
        CursorIcon, Fullscreen as WryFullscreen, Icon, ProgressBarState, RGBA, ResizeDirection,
        Theme, UserAttentionType, Window, WindowId, WindowSizeConstraints,
    },
};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use wry::{RGBA as WebViewRGBA, Rect, RequestAsyncResponder, WebView};

#[cfg(target_os = "ios")]
use objc2::rc::Retained;
#[cfg(target_os = "ios")]
use objc2_ui_kit::UIView;
#[cfg(target_os = "ios")]
use tao::platform::ios::WindowExtIOS;

/// Macro to generate proxy methods that forward to DesktopService methods.
macro_rules! proxy_desktop_service_method {
    ($(
        $(#[$meta:meta])*
        fn $name:ident(&self $(, $arg:ident : $arg_ty:ty)* ) $(-> $ret:ty)?;
    )*) => {
        $(
            $(#[$meta])*
            pub fn $name(&self $(, $arg: $arg_ty)*) $(-> $ret)? {
                self.run_with_desktop_service_blocking(move |desktop| desktop.$name($($arg),*))
            }
        )*
    };
}

/// Macro to generate proxy methods that forward to Window methods (via desktop.window).
macro_rules! proxy_window_method {
    ($(
        $(#[$meta:meta])*
        fn $name:ident(&self $(, $arg:ident : $arg_ty:ty)* ) $(-> $ret:ty)?;
    )*) => {
        $(
            $(#[$meta])*
            pub fn $name(&self $(, $arg: $arg_ty)*) $(-> $ret)? {
                self.run_with_desktop_service_blocking(move |desktop| desktop.window.$name($($arg),*))
            }
        )*
    };
}

/// Macro to generate proxy methods that forward to WebView methods (via desktop.webview).
macro_rules! proxy_webview_method {
    ($(
        $(#[$meta:meta])*
        fn $name:ident(&self $(, $arg:ident : $arg_ty:ty)* ) $(-> $ret:ty)?;
    )*) => {
        $(
            $(#[$meta])*
            pub fn $name(&self $(, $arg: $arg_ty)*) $(-> $ret)? {
                self.run_with_desktop_service_blocking(move |desktop| desktop.webview.$name($($arg),*))
            }
        )*
    };
}

/// Get an imperative handle to the current window without using a hook
///
/// ## Panics
///
/// This function will panic if it is called outside of the context of a Dioxus App.
pub fn window() -> DesktopContext {
    dioxus_core::consume_context()
}

#[derive(Clone)]
pub(crate) struct DesktopContextInner {
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
    /// Channel to send events to the DOM thread for the inverted callback pattern.
    dom_tx: UnboundedSender<VirtualDomEvent>,
}

/// A handle to the [`DesktopService`] for the current VirtualDom thread.
#[derive(Clone)]
pub struct DesktopContext {
    inner: DesktopContextInner,
    // Keep this handle on the thread it was created for. Blocking desktop calls from the Tao event
    // loop thread can deadlock because they send work to that same event loop and wait for it.
    unsend: PhantomData<*const ()>,
}

impl DesktopContext {
    fn from_inner(inner: DesktopContextInner) -> Self {
        Self {
            inner,
            unsend: PhantomData,
        }
    }

    /// Create a new [`DesktopContext`] from an event loop proxy.
    ///
    /// # Arguments
    ///
    /// * `proxy` - The event loop proxy for sending events to the main thread
    /// * `window_id` - The window ID this proxy is associated with
    /// * `dom_tx` - Channel to send events to the DOM thread
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let ctx = DesktopContext::new(event_loop_proxy, window_id, dom_tx);
    /// ```
    pub(crate) fn new(
        proxy: EventLoopProxy<UserWindowEvent>,
        window_id: WindowId,
        dom_tx: UnboundedSender<VirtualDomEvent>,
    ) -> Self {
        Self::from_inner(DesktopContextInner {
            proxy,
            window_id,
            dom_tx,
        })
    }

    /// Run a closure on the main thread and block until it returns its result.
    ///
    /// Dioxus desktop runs your components on a dedicated thread, while the OS event loop and every
    /// native window live on the *main* thread. Some platform and FFI APIs may only be called from
    /// the main thread; this method ships the closure over there, runs it, and hands the result
    /// back.
    ///
    /// The closure and its return value must be `Send` because they cross the thread boundary. The
    /// closure takes no arguments — use the methods on [`DesktopContext`] (such as
    /// [`set_title`](Self::set_title)) when you need to touch the window or webview.
    ///
    /// # Panics
    ///
    /// Panics if the event loop has been dropped.
    ///
    /// Do **not** call this from code that already runs on the main thread (such as the
    /// target-taking form of [`use_wry_event_handler`](crate::use_wry_event_handler)): it would
    /// block the event loop waiting on itself and deadlock.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let answer = window().run_on_main_thread(|| {
    ///     // main-thread-only FFI goes here
    ///     6 * 7
    /// });
    /// assert_eq!(answer, 42);
    /// ```
    pub fn run_on_main_thread<T, F>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.run_with_desktop_service_blocking(move |_| f())
    }

    /// Run a closure on the main thread with access to this window's [`DesktopService`], returning
    /// a receiver for the result. Await it or call
    /// [`blocking_recv`](tokio::sync::oneshot::Receiver::blocking_recv).
    ///
    /// # Panics
    ///
    /// Panics if the event loop has been dropped.
    fn run_with_desktop_service<T, F>(&self, f: F) -> oneshot::Receiver<T>
    where
        T: Send + 'static,
        F: FnOnce(&DesktopService) -> T + Send + 'static,
    {
        let (callback, receiver) = DesktopServiceCallback::new(f);

        self.inner
            .proxy
            .send_event(UserWindowEvent::run_with_desktop_service(
                self.inner.window_id,
                callback,
            ))
            .expect("Event loop has been dropped");

        receiver
    }

    fn run_with_desktop_service_blocking<T, F>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&DesktopService) -> T + Send + 'static,
    {
        let (callback, receiver) = DesktopServiceCallback::new_blocking(f);

        self.inner
            .proxy
            .send_event(UserWindowEvent::run_with_desktop_service(
                self.inner.window_id,
                callback,
            ))
            .expect("Event loop has been dropped");

        receiver.recv().expect("Failed to receive result")
    }

    proxy_desktop_service_method! {
        /// Trigger the drag-window event.
        ///
        /// Moves the window with the left mouse button until the button is released.
        fn drag(&self);

        /// Toggle whether the window is maximized or not.
        fn toggle_maximized(&self);

        /// Set the close behavior of this window.
        ///
        /// By default, windows close when the user clicks the close button.
        /// If this is set to `WindowCloseBehaviour::WindowHides`, the window will hide instead of closing.
        fn set_close_behavior(&self, behaviour: WindowCloseBehaviour);

        /// Close this window.
        fn close(&self);

        /// Close a particular window, given its ID.
        fn close_window(&self, id: WindowId);

        /// Change window to fullscreen.
        fn set_fullscreen(&self, fullscreen: bool);

        /// Launch print modal.
        fn print(&self);

        /// Set the zoom level of the webview.
        fn set_zoom_level(&self, level: f64);

        /// Opens DevTool window.
        fn devtool(&self);

        /// Remove a global shortcut.
        fn remove_shortcut(&self, id: ShortcutHandle);

        /// Remove all global shortcuts.
        fn remove_all_shortcuts(&self);
    }

    /// Start the creation of a new window using a [`VirtualDom`] and window builder.
    ///
    /// Returns a future that resolves to the [`DesktopContext`] for the new window.
    ///
    /// Note: `Config` is not `Send`, so this method takes a closure that creates the config
    /// on the main thread instead of accepting it directly. The [`VirtualDom`] stays on the DOM
    /// thread where this method is called.
    pub fn new_window(
        &self,
        dom: VirtualDom,
        make_cfg: impl FnOnce() -> Config + Send + 'static,
    ) -> PendingDesktopContext {
        let pending_dom = reserve_pending_dom(dom);
        let (sender, receiver) = futures_channel::oneshot::channel();

        let _ = self.run_with_desktop_service(move |desktop| {
            desktop.new_window_from_pending_dom(pending_dom, make_cfg(), sender);
        });

        PendingDesktopContext { receiver }
    }

    /// Returns the unique identifier of the window.
    pub fn window_id(&self) -> WindowId {
        self.inner.window_id
    }

    pub(crate) fn dom_event_sender(&self) -> UnboundedSender<VirtualDomEvent> {
        self.inner.dom_tx.clone()
    }

    proxy_window_method! {
        /// Returns the scale factor of the window.
        fn scale_factor(&self) -> f64;

        /// Emits a [`Event::RedrawRequested`] event.
        fn request_redraw(&self);

        /// Returns the position of the top-left hand corner of the window's client area.
        fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError>;

        /// Returns the position of the top-left hand corner of the window.
        fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError>;

        /// Modifies the position of the window.
        fn set_outer_position(&self, position: Position);

        /// Returns the size of the window's client area.
        fn inner_size(&self) -> PhysicalSize<u32>;

        /// Modifies the inner size of the window.
        fn set_inner_size(&self, size: Size);

        /// Returns the size of the entire window.
        fn outer_size(&self) -> PhysicalSize<u32>;

        /// Sets a minimum dimension size for the window.
        fn set_min_inner_size(&self, min_size: Option<Size>);

        /// Sets a maximum dimension size for the window.
        fn set_max_inner_size(&self, max_size: Option<Size>);

        /// Sets inner size constraints for the window.
        fn set_inner_size_constraints(&self, constraints: WindowSizeConstraints);

        /// Gets the current title of the window.
        fn title(&self) -> String;

        /// Modifies the window's visibility.
        fn set_visible(&self, visible: bool);

        /// Gets the window's current visibility state.
        fn is_visible(&self) -> bool;

        /// Brings the window to the front and sets input focus.
        fn set_focus(&self);

        /// Sets whether the window is focusable.
        fn set_focusable(&self, focusable: bool);

        /// Returns whether the window is focused.
        fn is_focused(&self) -> bool;

        /// Sets whether the window is resizable.
        fn set_resizable(&self, resizable: bool);

        /// Returns whether the window is resizable.
        fn is_resizable(&self) -> bool;

        /// Sets whether the window is minimizable.
        fn set_minimizable(&self, minimizable: bool);

        /// Returns whether the window is minimizable.
        fn is_minimizable(&self) -> bool;

        /// Sets whether the window is maximizable.
        fn set_maximizable(&self, maximizable: bool);

        /// Returns whether the window is maximizable.
        fn is_maximizable(&self) -> bool;

        /// Sets whether the window is closable.
        fn set_closable(&self, closable: bool);

        /// Returns whether the window is closable.
        fn is_closable(&self) -> bool;

        /// Sets the window to minimized or back.
        fn set_minimized(&self, minimized: bool);

        /// Returns whether the window is minimized.
        fn is_minimized(&self) -> bool;

        /// Sets the window to maximized or back.
        fn set_maximized(&self, maximized: bool);

        /// Returns whether the window is maximized.
        fn is_maximized(&self) -> bool;

        /// Turn window decorations on or off.
        fn set_decorations(&self, decorations: bool);

        /// Returns whether the window is decorated.
        fn is_decorated(&self) -> bool;

        /// Change whether the window is always on bottom.
        fn set_always_on_bottom(&self, always_on_bottom: bool);

        /// Change whether the window is always on top.
        fn set_always_on_top(&self, always_on_top: bool);

        /// Returns whether the window is always on top.
        fn is_always_on_top(&self) -> bool;

        /// Sets the window icon.
        fn set_window_icon(&self, window_icon: Option<Icon>);

        /// Sets the location of the IME candidate box.
        fn set_ime_position(&self, position: Position);

        /// Sets the taskbar progress state.
        fn set_progress_bar(&self, progress: ProgressBarState);

        /// Requests user attention to the window.
        fn request_user_attention(&self, request_type: Option<UserAttentionType>);

        /// Returns the current window theme.
        fn theme(&self) -> Theme;

        /// Sets the window theme.
        fn set_theme(&self, theme: Option<Theme>);

        /// Prevents the window contents from being captured by other apps.
        fn set_content_protection(&self, enabled: bool);

        /// Sets whether the window should be visible on all workspaces.
        fn set_visible_on_all_workspaces(&self, visible: bool);

        /// Sets the window background color.
        fn set_background_color(&self, color: Option<RGBA>);

        /// Gets the window's current fullscreen state.
        fn fullscreen(&self) -> Option<WryFullscreen>;

        /// Modifies the cursor icon of the window.
        fn set_cursor_icon(&self, cursor: CursorIcon);

        /// Changes the position of the cursor in window coordinates.
        fn set_cursor_position(&self, position: Position) -> Result<(), ExternalError>;

        /// Grabs the cursor, preventing it from leaving the window.
        fn set_cursor_grab(&self, grab: bool) -> Result<(), ExternalError>;

        /// Modifies the cursor's visibility.
        fn set_cursor_visible(&self, visible: bool);

        /// Moves the window with the left mouse button until the button is released.
        fn drag_window(&self) -> Result<(), ExternalError>;

        /// Resizes the window with the left mouse button until the button is released.
        fn drag_resize_window(&self, direction: ResizeDirection) -> Result<(), ExternalError>;

        /// Modifies whether the window catches cursor events.
        fn set_ignore_cursor_events(&self, ignore: bool) -> Result<(), ExternalError>;

        /// Returns the cursor position in window coordinates.
        fn cursor_position(&self) -> Result<PhysicalPosition<f64>, ExternalError>;

        /// Returns the monitor on which the window currently resides.
        fn current_monitor(&self) -> Option<MonitorHandle>;

        /// Returns the primary monitor of the system.
        fn primary_monitor(&self) -> Option<MonitorHandle>;

        /// Returns the monitor that contains the given point.
        fn monitor_from_point(&self, x: f64, y: f64) -> Option<MonitorHandle>;
    }

    /// Modifies the title of the window.
    pub fn set_title(&self, title: &str) {
        let title = title.to_string();
        self.run_with_desktop_service_blocking(move |desktop| desktop.window.set_title(&title))
    }

    /// Returns the list of all the monitors available on the system.
    pub fn available_monitors(&self) -> Vec<MonitorHandle> {
        self.run_with_desktop_service_blocking(|desktop| {
            desktop.window.available_monitors().collect()
        })
    }

    proxy_webview_method! {
        /// Get the current URL of the webview.
        fn url(&self) -> wry::Result<String>;

        /// Reload the current page.
        fn reload(&self) -> wry::Result<()>;

        /// Set the zoom level of the webview.
        fn zoom(&self, scale_factor: f64) -> wry::Result<()>;

        /// Move focus from the webview back to the parent window.
        fn focus_parent(&self) -> wry::Result<()>;

        /// Clear all browsing data.
        fn clear_all_browsing_data(&self) -> wry::Result<()>;

        /// Open the developer tools window.
        fn open_devtools(&self);

        /// Close the developer tools window.
        fn close_devtools(&self);

        /// Check if the developer tools window is open.
        fn is_devtools_open(&self) -> bool;
    }

    /// Set the background color of the webview.
    pub fn set_webview_background_color(&self, background_color: WebViewRGBA) -> wry::Result<()> {
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.webview.set_background_color(background_color)
        })
    }

    /// Get the bounds of the webview.
    pub fn webview_bounds(&self) -> wry::Result<Rect> {
        self.run_with_desktop_service_blocking(|desktop| desktop.webview.bounds())
    }

    /// Set the bounds of the webview.
    pub fn set_webview_bounds(&self, bounds: Rect) -> wry::Result<()> {
        self.run_with_desktop_service_blocking(move |desktop| desktop.webview.set_bounds(bounds))
    }

    /// Set the visibility of the webview.
    pub fn set_webview_visible(&self, visible: bool) -> wry::Result<()> {
        self.run_with_desktop_service_blocking(move |desktop| desktop.webview.set_visible(visible))
    }

    /// Focus the webview.
    pub fn webview_focus(&self) -> wry::Result<()> {
        self.run_with_desktop_service_blocking(|desktop| desktop.webview.focus())
    }

    /// Launch the print modal for the webview content.
    pub fn webview_print(&self) -> wry::Result<()> {
        self.run_with_desktop_service_blocking(|desktop| desktop.webview.print())
    }

    /// Load a URL in the webview.
    pub fn load_url(&self, url: &str) -> wry::Result<()> {
        let url = url.to_string();
        self.run_with_desktop_service_blocking(move |desktop| desktop.webview.load_url(&url))
    }

    /// Load a URL with custom headers in the webview.
    pub fn load_url_with_headers(
        &self,
        url: &str,
        headers: wry::http::HeaderMap,
    ) -> wry::Result<()> {
        let url = url.to_string();
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.webview.load_url_with_headers(&url, headers)
        })
    }

    /// Load HTML content directly into the webview.
    pub fn load_html(&self, html: &str) -> wry::Result<()> {
        let html = html.to_string();
        self.run_with_desktop_service_blocking(move |desktop| desktop.webview.load_html(&html))
    }

    /// Evaluate JavaScript in the webview.
    pub fn evaluate_script(&self, js: &str) -> wry::Result<()> {
        let js = js.to_string();
        self.run_with_desktop_service_blocking(move |desktop| desktop.webview.evaluate_script(&js))
    }

    /// Create a wry event handler that listens for wry events.
    ///
    /// This is the thread-safe version that accepts `Send` closures, allowing
    /// event handlers to be created from any thread.
    ///
    /// See [`DesktopService::create_wry_event_handler`] for more details.
    pub fn create_wry_event_handler(
        &self,
        handler: impl FnMut(&Event<()>, &EventLoopWindowTarget<UserWindowEvent>) + Send + 'static,
    ) -> WryEventHandler {
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.create_wry_event_handler(handler)
        })
    }

    pub(crate) fn create_wry_event_handler_with_user_event(
        &self,
        handler: impl FnMut(&Event<UserWindowEvent>, &EventLoopWindowTarget<UserWindowEvent>)
        + Send
        + 'static,
    ) -> WryEventHandler {
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.create_wry_event_handler_with_user_event(handler)
        })
    }

    /// Register a wry event handler whose closure stays on the VirtualDom thread (no `Send` bound).
    ///
    /// The closure itself lives in this window's [`DomCallbackRegistry`]; this method only sets up
    /// a small `Send` forwarder on the main thread. When a wry event arrives, the forwarder converts
    /// events Tao can safely own into `'static` events and queues them for the DOM thread without
    /// blocking the event loop. Tao events that borrow from the event loop, such as
    /// [`WindowEvent::ScaleFactorChanged`], are left untouched and not forwarded.
    ///
    /// [`DomCallbackRegistry`]: crate::dom_thread::DomCallbackRegistry
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub(crate) fn create_wry_event_handler_forwarding(
        &self,
        handler: impl FnMut(&Event<()>) + 'static,
    ) -> WryEventHandler {
        let dom_tx = self.inner.dom_tx.clone();
        let dom_handler = {
            use dioxus_core::{Runtime, consume_context, current_scope_id};

            let registry: SharedCallbackRegistry = consume_context();
            let runtime = Runtime::current();
            let scope_id = current_scope_id();
            let mut handler = handler;
            registry
                .borrow_mut()
                .register_wry_event_handler(Box::new(move |event| {
                    runtime.in_scope(scope_id, || handler(&event));
                    event
                }))
        };
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.create_raw_wry_event_handler(move |event, _target| {
                use crate::dom_thread::DomCallbackRegistry;

                let event = match event.map_nonuser_event() {
                    Ok(event) => event,
                    Err(user_event) => return user_event,
                };

                if matches!(
                    event,
                    Event::WindowEvent {
                        event: WindowEvent::ScaleFactorChanged { .. },
                        ..
                    }
                ) {
                    return event
                        .map_nonuser_event()
                        .expect("non-user event stays non-user when static forwarding is skipped");
                }

                let event = event
                    .to_static()
                    .expect("only ScaleFactorChanged contains non-static data");
                let return_event = event.clone();
                let callback = move |registry: &mut DomCallbackRegistry| {
                    registry.invoke_wry_event_handler(dom_handler, event);
                };
                let _ = dom_tx.send(VirtualDomEvent::RunCallback(Box::new(callback)));

                return_event
                    .map_nonuser_event()
                    .expect("non-user event stays non-user after being queued for the handler")
            })
        })
        .with_dom_handler(dom_handler)
    }

    /// Remove a wry event handler created with [`Self::create_wry_event_handler`].
    pub fn remove_wry_event_handler(&self, id: WryEventHandler) {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        if let Some(dom_handler) = id.dom_handler {
            if let Some(registry) = dioxus_core::try_consume_context::<SharedCallbackRegistry>() {
                registry.borrow_mut().remove_event_handler(dom_handler);
            }
        }

        self.run_with_desktop_service_blocking(move |desktop| desktop.remove_wry_event_handler(id))
    }

    /// Register an asset handler using the inverted callback pattern.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for this handler
    /// * `handler` - The handler function (does not need to be `Send`)
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let ctx = consume_context::<DesktopContext>();
    /// ctx.register_asset_handler("my-protocol", |req, resp| {
    ///     // Handle asset request
    /// });
    /// ```
    pub fn register_asset_handler(
        &self,
        name: impl Into<String>,
        handler: impl Fn(AssetRequest, RequestAsyncResponder) + 'static,
    ) {
        let registry: SharedCallbackRegistry = dioxus_core::consume_context();
        let name = name.into();

        // Store the handler in the DOM registry
        registry
            .borrow_mut()
            .register_asset_handler(name.clone(), Box::new(handler));

        // Set up forwarding on the main thread
        let dom_tx = self.inner.dom_tx.clone();
        let handler_name = name.clone();
        self.run_with_desktop_service_blocking(move |desktop| {
            // Register a forwarder that sends requests to the DOM thread
            desktop.asset_handlers.register_handler(
                name,
                Callback::new(move |(req, resp): (AssetRequest, RequestAsyncResponder)| {
                    let handler_name = handler_name.clone();
                    let _ = dom_tx.send(VirtualDomEvent::RunCallback(Box::new(move |registry| {
                        registry.invoke_asset_handler(&handler_name, req, resp);
                    })));
                }),
            );
        });
    }

    /// Create a global shortcut using the inverted callback pattern.
    ///
    /// The callback stays on the DOM thread (no `Send` requirement). When the
    /// shortcut is triggered, the event is forwarded to the DOM thread.
    ///
    /// # Arguments
    ///
    /// * `hotkey` - The key combination for the shortcut
    /// * `callback` - The callback function (does not need to be `Send`)
    ///
    /// # Returns
    ///
    /// A tuple of `(ShortcutHandle, DomCallbackId)` on success. The `ShortcutHandle`
    /// can be used with `remove_shortcut` on the main thread, and `DomCallbackId`
    /// can be used to remove the callback from the DOM registry.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let ctx = consume_context::<DesktopContext>();
    /// let (handle, dom_id) = ctx.create_shortcut(hotkey, |state| {
    ///     // Handle shortcut
    /// })?;
    /// ```
    pub fn create_shortcut(
        &self,
        hotkey: HotKey,
        callback: impl FnMut(HotKeyState) + 'static,
    ) -> Result<(ShortcutHandle, DomCallbackId), ShortcutRegistryError> {
        let registry: SharedCallbackRegistry = dioxus_core::consume_context();

        // Store the callback in the DOM registry
        let dom_id = registry
            .borrow_mut()
            .register_shortcut_callback(Box::new(callback));

        // Set up forwarding on the main thread
        let dom_tx = self.inner.dom_tx.clone();
        let result = self.run_with_desktop_service_blocking(move |desktop| {
            desktop.create_shortcut(hotkey, move |state| {
                let _ = dom_tx.send(VirtualDomEvent::RunCallback(Box::new(move |registry| {
                    registry.invoke_shortcut_callback(dom_id, state);
                })));
            })
        });

        match result {
            Ok(handle) => Ok((handle, dom_id)),
            Err(e) => {
                // Remove the callback from the DOM registry since main thread registration failed
                registry.borrow_mut().remove_shortcut_callback(dom_id);
                Err(e)
            }
        }
    }

    /// Remove an asset handler by name.
    ///
    /// This removes the handler from both the DOM registry and the main thread.
    pub fn remove_asset_handler(&self, name: &str) {
        let registry: SharedCallbackRegistry = dioxus_core::consume_context();
        registry.borrow_mut().remove_asset_handler(name);

        let name = name.to_string();
        self.run_with_desktop_service_blocking(move |desktop| {
            desktop.asset_handlers.remove_handler(&name);
        });
    }

    /// Remove a shortcut that was created with the inverted callback pattern (`create_shortcut`).
    ///
    /// This removes both the main thread shortcut and the DOM callback.
    pub fn remove_dom_shortcut(&self, handle: ShortcutHandle, dom_id: DomCallbackId) {
        let registry: SharedCallbackRegistry = dioxus_core::consume_context();
        registry.borrow_mut().remove_shortcut_callback(dom_id);
        handle.remove();
    }
}

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

    pub(crate) asset_handlers: AssetHandlerRegistry,
    pub(crate) close_behaviour: Rc<Cell<WindowCloseBehaviour>>,

    /// Channel to send events to the DOM thread for the inverted callback pattern.
    pub(crate) dom_tx: UnboundedSender<VirtualDomEvent>,

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
        close_behaviour: WindowCloseBehaviour,
        dom_tx: UnboundedSender<VirtualDomEvent>,
    ) -> Self {
        Self {
            window,
            webview,
            shared,
            asset_handlers,
            close_behaviour: Rc::new(Cell::new(close_behaviour)),
            dom_tx,
            #[cfg(target_os = "ios")]
            views: Default::default(),
        }
    }

    /// Start the creation of a new window using a component function and window builder.
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
    /// let window = dioxus::desktop::window().new_window(dom, Default::default).await;
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
    pub fn new_window(
        &self,
        dom: impl FnOnce() -> VirtualDom + Send + 'static,
        cfg: Config,
    ) -> PendingDesktopContext {
        let (window, context) = PendingWebview::new(cfg, Box::new(dom));

        self.queue_new_window(window);

        context
    }

    pub(crate) fn new_window_from_pending_dom(
        &self,
        dom: PendingDomId,
        cfg: Config,
        sender: futures_channel::oneshot::Sender<DesktopContextInner>,
    ) {
        let window = PendingWebview::from_pending_dom(cfg, dom, sender);
        self.queue_new_window(window);
    }

    fn queue_new_window(&self, window: PendingWebview) {
        self.shared
            .proxy
            .send_event(UserWindowEvent::new_window())
            .unwrap();

        self.shared.pending_webviews.borrow_mut().push(window);
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
            .send_event(UserWindowEvent::close_window(self.id()));
    }

    /// Close a particular window, given its ID
    pub fn close_window(&self, id: WindowId) {
        let _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent::close_window(id));
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
        mut handler: impl FnMut(&Event<()>, &EventLoopWindowTarget<UserWindowEvent>) + 'static,
    ) -> WryEventHandler {
        self.shared
            .event_handlers
            .add(self.window.id(), move |event, target| {
                handler(&event, target);
            })
    }

    pub(crate) fn create_wry_event_handler_with_user_event(
        &self,
        handler: impl for<'a> FnMut(
            &Event<'a, UserWindowEvent>,
            &EventLoopWindowTarget<UserWindowEvent>,
        ) + 'static,
    ) -> WryEventHandler {
        self.shared
            .event_handlers
            .add_with_user_event(self.window.id(), handler)
    }

    pub(crate) fn create_raw_wry_event_handler(
        &self,
        mut handler: impl for<'a> FnMut(
            Event<'a, UserWindowEvent>,
            &EventLoopWindowTarget<UserWindowEvent>,
        ) -> Event<'a, UserWindowEvent>
        + 'static,
    ) -> WryEventHandler {
        self.shared
            .event_handlers
            .add_raw(self.window.id(), move |event, target| {
                handler(event, target)
            })
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

    /// Get a proxy to this [`DesktopService`] for the current VirtualDom thread.
    ///
    /// The proxy runs closures on the main thread via
    /// [`run_on_main_thread`](DesktopContext::run_on_main_thread) and exposes the window/webview
    /// methods on [`DesktopContext`].
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let proxy = window().proxy();
    ///
    /// let title = proxy.title();
    /// println!("Window title: {}", title);
    /// ```
    pub fn proxy(&self) -> DesktopContext {
        DesktopContext::from_inner(self.proxy_inner())
    }

    pub(crate) fn proxy_inner(&self) -> DesktopContextInner {
        DesktopContextInner {
            proxy: self.shared.proxy.clone(),
            window_id: self.window.id(),
            dom_tx: self.dom_tx.clone(),
        }
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
/// let pending_context = dioxus::desktop::window().new_window(dom, Default::default);
///
/// // Wait for the context to be created
/// let window = pending_context.await;
///
/// // Now control the window
/// window.set_fullscreen(true);
/// # }
/// ```
pub struct PendingDesktopContext {
    pub(crate) receiver: futures_channel::oneshot::Receiver<DesktopContextInner>,
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
        self.receiver.await.map(DesktopContext::from_inner)
    }
}

impl IntoFuture for PendingDesktopContext {
    type Output = DesktopContext;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.resolve())
    }
}
