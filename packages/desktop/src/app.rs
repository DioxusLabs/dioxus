use crate::{
    config::{Config, WindowCloseBehaviour},
    desktop_state::{DesktopAppContext, WindowCloseRequestResult},
    ipc::{IpcMessage, UserWindowEvent},
    query::QueryResult,
    waker::create_dom_waker,
    webview::WebviewInstance,
};
use dioxus_core::{RenderTargetId, ScopeId, VirtualDom, provide_context};
use futures_util::{FutureExt, pin_mut};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
    task::Waker,
    time::Duration,
};
use tao::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::WindowId,
};

/// The single top-level object that manages the event loop, VirtualDom, and running windows.
pub(crate) struct App {
    pub(crate) dom: VirtualDom,
    pub(crate) initial_dom_rebuild_done: bool,

    // Stuff we need mutable access to
    pub(crate) control_flow: ControlFlow,
    pub(crate) exit_on_last_window_close: bool,
    pub(crate) disable_dma_buf_on_wayland: bool,
    pub(crate) webviews: HashMap<WindowId, WebviewInstance>,
    pub(crate) float_all: bool,
    pub(crate) show_devtools: bool,
    pub(crate) tray_icon_show_window_on_click: bool,
    pub(crate) dom_waker: Waker,

    /// App-wide state shared by every desktop window.
    pub(crate) app_context: Rc<DesktopAppContext>,
}

impl App {
    pub fn new(mut cfg: Config, virtual_dom: VirtualDom) -> (EventLoop<UserWindowEvent>, Self) {
        let event_loop = cfg
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoopBuilder::<UserWindowEvent>::with_user_event().build());

        let tray_icon_show_window_on_click = cfg.tray_icon_show_window_on_click;
        let proxy = event_loop.create_proxy();
        let dom_waker = create_dom_waker(proxy.clone());

        let app = Self {
            exit_on_last_window_close: cfg.exit_on_last_window_close,
            disable_dma_buf_on_wayland: cfg.disable_dma_buf_on_wayland,
            webviews: HashMap::new(),
            control_flow: ControlFlow::Wait,
            dom: virtual_dom,
            initial_dom_rebuild_done: false,
            float_all: false,
            show_devtools: false,
            tray_icon_show_window_on_click,
            dom_waker,
            app_context: Rc::new(DesktopAppContext::new(proxy, event_loop.clone())),
        };

        // Set the event converter
        dioxus_html::set_event_converter(Box::new(crate::events::SerializedHtmlEventConverter));

        // Wire up the global hotkey handler
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        app.set_global_hotkey_handler();

        // Wire up the menubar receiver - this way any component can key into the menubar actions
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        app.set_menubar_receiver();

        // Wire up the tray icon receiver - this way any component can key into the menubar actions
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        app.set_tray_icon_receiver();

        // Allow hotreloading to work - but only in debug mode
        #[cfg(all(feature = "devtools", debug_assertions))]
        app.connect_hotreload();

        #[cfg(debug_assertions)]
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        app.connect_preserve_window_state_handler();

        // Make sure to disable DMA buffer rendering on Linux Wayland sessions
        app.disable_dma_buf();

        (event_loop, app)
    }

    pub fn tick(&mut self, window_event: &Event<'_, UserWindowEvent>) {
        self.control_flow = ControlFlow::Wait;
        self.app_context
            .event_handlers
            .apply_event(window_event, &self.app_context.target);
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn handle_global_hotkey(&self, event: global_hotkey::GlobalHotKeyEvent) {
        self.app_context.shortcut_manager.call_handlers(event);
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn handle_menu_event(&mut self, event: muda::MenuEvent) {
        match event.id().0.as_str() {
            "dioxus-float-top" => {
                for app_webview in self.webviews.values() {
                    app_webview
                        .desktop_context
                        .window
                        .set_always_on_top(self.float_all);
                }
                self.float_all = !self.float_all;
            }
            "dioxus-toggle-dev-tools" => {
                self.show_devtools = !self.show_devtools;
                for app_webview in self.webviews.values() {
                    let wv = &app_webview.desktop_context.webview;
                    if self.show_devtools {
                        wv.open_devtools();
                    } else {
                        wv.close_devtools();
                    }
                }
            }
            _ => (),
        }
    }
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn handle_tray_menu_event(&mut self, event: tray_icon::menu::MenuEvent) {
        _ = event;
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn handle_tray_icon_event(&mut self, event: tray_icon::TrayIconEvent) {
        if let tray_icon::TrayIconEvent::Click {
            id: _,
            position: _,
            rect: _,
            button,
            button_state: _,
        } = event
        {
            if button == tray_icon::MouseButton::Left && self.tray_icon_show_window_on_click {
                for app_webview in self.webviews.values() {
                    app_webview.desktop_context.window.set_visible(true);
                    app_webview.desktop_context.window.set_focus();
                }
            }
        }
    }

    #[cfg(all(feature = "devtools", debug_assertions))]
    pub fn connect_hotreload(&self) {
        let proxy = self.app_context.proxy.clone();
        dioxus_devtools::connect(move |msg| {
            _ = proxy.send_event(UserWindowEvent::HotReloadEvent(msg));
        })
    }

    pub fn handle_new_window(&mut self) {
        for pending_webview in self.app_context.drain_pending_webviews() {
            let app_webview = pending_webview.create_window(&mut self.dom, &self.app_context);
            let id = app_webview.desktop_context.window.id();
            self.webviews.insert(id, app_webview);
            self.schedule_poll();
        }
    }

    pub fn handle_close_requested(&mut self, id: WindowId) {
        let Some(window) = self.webviews.get(&id) else {
            // If the window is not found, we can just return
            return;
        };

        match window.desktop_context.close_behaviour.get() {
            // If the window is just set to hide when closed, we can just hide it
            WindowCloseBehaviour::WindowHides => {
                window.desktop_context.window.set_visible(false);
            }

            // Component-owned windows render out before native teardown. Root
            // and unowned windows can be dropped here.
            WindowCloseBehaviour::WindowCloses => {
                #[cfg(debug_assertions)]
                self.persist_window_state();

                match window.desktop_context.request_window_close() {
                    WindowCloseRequestResult::DeferredToComponent => {}
                    WindowCloseRequestResult::CloseImmediately => self.destroy_window(id),
                }
            }
        };
    }

    pub fn window_destroyed(&mut self, id: WindowId) {
        self.destroy_window(id);
    }

    /// Tear down one webview after its Dioxus owner has released the target, or
    /// immediately for root/unowned windows.
    pub(crate) fn destroy_window(&mut self, id: WindowId) {
        let Some(app_webview) = self.webviews.remove(&id) else {
            return;
        };
        app_webview.desktop_context.notify_window_destroyed();
        let target_id = app_webview.target_id();
        // A component-owned `Window` reclaims its own target when its portal is
        // torn down, so this is usually a no-op. When the OS destroys a window
        // (or the app is shutting down) before that teardown runs the portal is
        // still mounted; `remove_render_target` leaves such a target in place and
        // it is reclaimed when the runtime is dropped.
        self.dom.runtime().remove_render_target(target_id);

        if self.exit_on_last_window_close && self.webviews.is_empty() {
            self.control_flow = ControlFlow::Exit
        } else {
            // Removing the webview drops its `WryQueue`, including any
            // `edits_in_progress` receiver that was gating `poll_vdom`. The
            // webview's pending ack can no longer wake the event loop, so
            // schedule a poll to resume any VDOM work that was waiting on it
            // (for example effects queued by the window's `onclose`).
            self.schedule_poll();
        }
    }

    pub fn resize_window(&self, id: WindowId, size: PhysicalSize<u32>) {
        // TODO: the app layer should avoid directly manipulating the webview webview instance internals.
        // Window creation and modification is the responsibility of the webview instance so it makes sense to
        // encapsulate that there.
        if let Some(app_webview) = self.webviews.get(&id) {
            use wry::Rect;

            _ = app_webview.desktop_context.webview.set_bounds(Rect {
                position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(
                    size.width,
                    size.height,
                )),
            });
        }
    }

    pub fn handle_start_cause_init(&mut self) {
        self.provide_app_context();
        self.rebuild_dom();
        self.handle_new_window();
    }

    fn provide_app_context(&mut self) {
        let app_context = self.app_context.clone();
        self.dom.in_scope(ScopeId::ROOT, || {
            provide_context(app_context);
        });
    }

    pub fn handle_browser_open(&mut self, msg: IpcMessage) {
        if let Some(temp) = msg.params().as_object() {
            if temp.contains_key("href") {
                if let Some(href) = temp.get("href").and_then(|v| v.as_str()) {
                    if let Err(err) = webbrowser::open(href) {
                        tracing::error!("Failed to open URL: {}", err);
                    }
                }
            }
        }
    }

    /// The webview is finally loaded. Rebuild once, then start polling the
    /// shared VDOM.
    pub fn handle_initialize_msg(&mut self, id: WindowId) {
        if !self.webviews.contains_key(&id) {
            return;
        }

        if !self.initial_dom_rebuild_done {
            self.rebuild_dom();
            self.send_touched_edits();
        }

        self.schedule_poll();
    }

    pub fn handle_query_msg(&mut self, msg: IpcMessage, id: WindowId) {
        let Ok(result) = serde_json::from_value::<QueryResult>(msg.params()) else {
            return;
        };

        let Some(view) = self.webviews.get(&id) else {
            return;
        };

        view.desktop_context.query.send(result);
    }

    #[cfg(all(feature = "devtools", debug_assertions))]
    pub fn handle_hot_reload_msg(&mut self, msg: dioxus_devtools::DevserverMsg) {
        use std::time::Duration;

        use dioxus_devtools::DevserverMsg;

        // Amount of time that toats should be displayed.
        const TOAST_TIMEOUT: Duration = Duration::from_secs(2);
        const TOAST_TIMEOUT_LONG: Duration = Duration::from_secs(3600); // Duration::MAX is too long for JS.

        match msg {
            DevserverMsg::HotReload(hr_msg) => {
                if !self.webviews.is_empty() {
                    {
                        // This is a place where wry says it's threadsafe but it's actually not.
                        // If we're patching the app, we want to make sure it's not going to progress in the interim.
                        #[cfg(target_os = "android")]
                        let _lock = crate::android_sync_lock::android_runtime_lock();

                        dioxus_devtools::apply_changes(&self.dom, &hr_msg);
                    }

                    self.poll_vdom();
                }

                if !hr_msg.assets.is_empty() {
                    for app_webview in self.webviews.values_mut() {
                        app_webview.kick_stylsheets();
                    }
                }

                if hr_msg.jump_table.is_some()
                    && hr_msg.for_build_id == Some(dioxus_cli_config::build_id())
                {
                    self.send_toast_to_all(
                        "Hot-patch success!",
                        &format!("App successfully patched in {} ms", hr_msg.ms_elapsed),
                        "success",
                        TOAST_TIMEOUT,
                        false,
                    );
                }
            }
            DevserverMsg::FullReloadCommand => {
                self.send_toast_to_all(
                    "Successfully rebuilt.",
                    "Your app was rebuilt successfully and without error.",
                    "success",
                    TOAST_TIMEOUT,
                    true,
                );
            }
            DevserverMsg::FullReloadStart => self.send_toast_to_all(
                "Your app is being rebuilt.",
                "A non-hot-reloadable change occurred and we must rebuild.",
                "info",
                TOAST_TIMEOUT_LONG,
                false,
            ),
            DevserverMsg::FullReloadFailed => self.send_toast_to_all(
                "Oops! The build failed.",
                "We tried to rebuild your app, but something went wrong.",
                "error",
                TOAST_TIMEOUT_LONG,
                false,
            ),
            DevserverMsg::HotPatchStart => self.send_toast_to_all(
                "Hot-patching app...",
                "Hot-patching modified Rust code.",
                "info",
                TOAST_TIMEOUT_LONG,
                false,
            ),
            DevserverMsg::Shutdown => {
                self.control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    }

    #[cfg(all(feature = "devtools", debug_assertions))]
    fn send_toast_to_all(
        &self,
        header_text: &str,
        message: &str,
        level: &str,
        duration: Duration,
        after_reload: bool,
    ) {
        for app_webview in self.webviews.values() {
            app_webview.show_toast(header_text, message, level, duration, after_reload);
        }
    }

    fn schedule_poll(&self) {
        _ = self.app_context.proxy.send_event(UserWindowEvent::Poll);
    }

    /// Poll the shared VirtualDom until it is pending.
    ///
    /// The app-level waker is connected to the event loop, so async work wakes
    /// the app by scheduling another [`UserWindowEvent::Poll`].
    pub fn poll_vdom(&mut self) {
        let dom_waker = self.dom_waker.clone();
        let mut cx = std::task::Context::from_waker(&dom_waker);

        loop {
            if self.poll_webview_queues(&mut cx) {
                return;
            }

            {
                // lock the hack-ed in lock sync wry has some thread-safety issues with event handlers and async tasks
                #[cfg(target_os = "android")]
                let _lock = crate::android_sync_lock::android_runtime_lock();
                let fut = self.dom.wait_for_work();
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => return,
                }
            }

            // lock the hack-ed in lock sync wry has some thread-safety issues with event handlers
            #[cfg(target_os = "android")]
            let _lock = crate::android_sync_lock::android_runtime_lock();

            self.render_dom_immediate();
            self.send_touched_edits();
        }
    }

    /// Build the writer for one render pass: every webview's `WryQueue` keyed
    /// by its target id, with each queue's `touched` flag cleared so we can
    /// detect which targets receive writes during the pass.
    fn dom_writer(&self) -> BTreeMap<RenderTargetId, crate::edits::WryQueue> {
        self.webviews
            .values()
            .map(|app_webview| {
                app_webview.edits.wry_queue.clear_touched();
                (app_webview.target_id(), app_webview.edits.wry_queue.clone())
            })
            .collect()
    }

    fn rebuild_dom(&mut self) {
        let mut writer = self.dom_writer();
        self.dom.rebuild(&mut writer);
        self.initial_dom_rebuild_done = true;
    }

    fn render_dom_immediate(&mut self) {
        let mut writer = self.dom_writer();
        self.dom.render_immediate(&mut writer);
    }

    /// Flush queued edits for every webview whose `WryQueue` was touched during
    /// the preceding render pass. The diff writes directly into each registered
    /// queue (the `WriteMutations` impl on `WryQueue`), and `dom_writer` cleared
    /// every `touched` flag at the start of the pass, so a touched queue means
    /// "this webview received new edits to send".
    fn send_touched_edits(&self) {
        for app_webview in self.webviews.values() {
            if app_webview.edits.wry_queue.is_touched() {
                app_webview.edits.wry_queue.send_edits();
            }
        }
    }

    fn poll_webview_queues(&self, cx: &mut std::task::Context<'_>) -> bool {
        let mut has_pending_edits = false;

        for app_webview in self.webviews.values() {
            if app_webview
                .edits
                .wry_queue
                .poll_new_edits_location(cx)
                .is_ready()
            {
                _ = app_webview
                    .desktop_context
                    .webview
                    .evaluate_script(&format!(
                        "window.interpreter.waitForRequest(\"{edits_path}\", \"{expected_key}\");",
                        edits_path = app_webview.edits.wry_queue.edits_path(),
                        expected_key = app_webview.edits.wry_queue.required_server_key()
                    ));
            }

            if app_webview
                .edits
                .wry_queue
                .poll_edits_flushed(cx)
                .is_pending()
            {
                has_pending_edits = true;
            }
        }

        has_pending_edits
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn set_global_hotkey_handler(&self) {
        let receiver = self.app_context.proxy.clone();

        // The event loop becomes the hotkey receiver
        // This means we don't need to poll the receiver on every tick - we just get the events as they come in
        // This is a bit more efficient than the previous implementation, but if someone else sets a handler, the
        // receiver will become inert.
        global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |t| {
            // todo: should we unset the event handler when the app shuts down?
            _ = receiver.send_event(UserWindowEvent::GlobalHotKeyEvent(t));
        }));
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn set_menubar_receiver(&self) {
        let receiver = self.app_context.proxy.clone();

        // The event loop becomes the menu receiver
        // This means we don't need to poll the receiver on every tick - we just get the events as they come in
        // This is a bit more efficient than the previous implementation, but if someone else sets a handler, the
        // receiver will become inert.
        muda::MenuEvent::set_event_handler(Some(move |t| {
            // todo: should we unset the event handler when the app shuts down?
            _ = receiver.send_event(UserWindowEvent::MudaMenuEvent(t));
        }));
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn set_tray_icon_receiver(&self) {
        let receiver = self.app_context.proxy.clone();

        // The event loop becomes the menu receiver
        // This means we don't need to poll the receiver on every tick - we just get the events as they come in
        // This is a bit more efficient than the previous implementation, but if someone else sets a handler, the
        // receiver will become inert.
        tray_icon::TrayIconEvent::set_event_handler(Some(move |t| {
            // todo: should we unset the event handler when the app shuts down?
            _ = receiver.send_event(UserWindowEvent::TrayIconEvent(t));
        }));

        // for whatever reason they had to make it separate
        let receiver = self.app_context.proxy.clone();
        tray_icon::menu::MenuEvent::set_event_handler(Some(move |t| {
            // todo: should we unset the event handler when the app shuts down?
            _ = receiver.send_event(UserWindowEvent::TrayMenuEvent(t));
        }));
    }

    /// Do our best to preserve state about the window when the event loop is destroyed
    ///
    /// This will attempt to save the window position, size, and monitor into the environment before
    /// closing. This way, when the app is restarted, it can attempt to restore the window to the same
    /// position and size it was in before, making a better DX.
    pub(crate) fn handle_loop_destroyed(&self) {
        #[cfg(debug_assertions)]
        self.persist_window_state();
    }

    #[cfg(debug_assertions)]
    fn persist_window_state(&self) {
        if let Some(app_webview) = self.webviews.values().next() {
            let window = &app_webview.desktop_context.window;

            let Some(monitor) = window.current_monitor() else {
                return;
            };

            let Ok(position) = window.outer_position() else {
                return;
            };
            let (x, y) = if cfg!(target_os = "macos") {
                let position = position.to_logical::<i32>(window.scale_factor());
                (position.x, position.y)
            } else {
                (position.x, position.y)
            };

            let (width, height) = if cfg!(target_os = "macos") {
                let size = window.outer_size();
                let size = size.to_logical::<u32>(window.scale_factor());
                // This is to work around a bug in how tao handles inner_size on macOS
                // We *want* to use inner_size, but that's currently broken, so we use outer_size instead and then an adjustment
                //
                // https://github.com/tauri-apps/tao/issues/889
                let adjustment = if window.is_decorated() { 28 } else { 0 };
                (size.width, size.height.saturating_sub(adjustment))
            } else {
                let size = window.inner_size();
                (size.width, size.height)
            };

            let Some(monitor_name) = monitor.name() else {
                return;
            };

            let state = PreservedWindowState {
                x,
                y,
                width: width.max(200),
                height: height.max(200),
                monitor: monitor_name.to_string(),
            };

            // Yes... I know... we're loading a file that might not be ours... but it's a debug feature
            if let Ok(state) = serde_json::to_string(&state) {
                _ = std::fs::write(restore_file(), state);
            }
        }
    }

    /// Wire up a receiver to sigkill that lets us preserve the window state
    /// Whenever sigkill is sent, we shut down the app and save the window state
    #[cfg(debug_assertions)]
    fn connect_preserve_window_state_handler(&self) {
        // TODO: make this work on windows
        #[cfg(unix)]
        {
            // Wire up the trap
            let target = self.app_context.proxy.clone();
            std::thread::spawn(move || {
                use signal_hook::consts::{SIGINT, SIGTERM};
                let sigkill = signal_hook::iterator::Signals::new([SIGTERM, SIGINT]);
                if let Ok(mut sigkill) = sigkill {
                    for _ in sigkill.forever() {
                        if target.send_event(UserWindowEvent::Shutdown).is_err() {
                            std::process::exit(0);
                        }

                        // give it a moment for the event to be processed
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            });
        }
    }

    /// Disable DMA buffer rendering on Linux Wayland sessions to avoid bugs with WebKitGTK
    fn disable_dma_buf(&self) {
        if cfg!(target_os = "linux") && self.disable_dma_buf_on_wayland {
            static INIT: std::sync::Once = std::sync::Once::new();
            INIT.call_once(|| {
                if std::path::Path::new("/dev/dri").exists()
                    && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland"
                {
                    // Gnome Webkit is currently buggy under Wayland and KDE, so we will run it with XWayland mode.
                    // See: https://github.com/DioxusLabs/dioxus/issues/3667
                    unsafe {
                        // Disable explicit sync for NVIDIA drivers on Linux when using Way
                        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                    }
                }
                unsafe {
                    std::env::set_var("GDK_BACKEND", "x11");
                }
            });
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct PreservedWindowState {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) monitor: String,
}

/// Return the location of a tempfile with our window state in it such that we can restore it later
pub(crate) fn restore_file() -> std::path::PathBuf {
    let dir = dioxus_cli_config::session_cache_dir().unwrap_or_else(std::env::temp_dir);
    dir.join("window-state.json")
}
