use crate::{
    config::{Config, WindowCloseBehaviour},
    desktop_context::DesktopContext,
    document::DesktopDocument,
    edits::EditWebsocket,
    event_handlers::{WindowCloseHandlers, WindowEventHandlers},
    ipc::{IpcMessage, UserWindowEvent},
    query::QueryResult,
    shortcut::ShortcutRegistry,
    webview::{PendingWebview, WebviewInstance},
};
use dioxus_core::{RenderTargetId, ScopeId, VirtualDom, provide_context};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use futures_util::{FutureExt, pin_mut};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
    time::Duration,
};
use tao::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::WindowId,
};

/// The single top-level object that manages all the running windows, assets, shortcuts, etc
pub(crate) struct App {
    // move the config into a cell so we can pop it out later to create the first window
    // iOS panics if we create a window before the event loop is started, so we toss them into a cell
    pub(crate) cfg: Cell<Option<Config>>,
    pub(crate) dom: VirtualDom,
    pub(crate) initial_dom_rebuild_done: bool,

    // Stuff we need mutable access to
    pub(crate) control_flow: ControlFlow,
    pub(crate) is_visible_before_start: bool,
    pub(crate) exit_on_last_window_close: bool,
    pub(crate) disable_dma_buf_on_wayland: bool,
    pub(crate) webviews: HashMap<WindowId, WebviewInstance>,
    pub(crate) float_all: bool,
    pub(crate) show_devtools: bool,
    pub(crate) tray_icon_show_window_on_click: bool,

    /// This single blob of state is shared between all the windows so they have access to the runtime state
    ///
    /// This includes stuff like the event handlers, shortcuts, etc as well as ways to modify *other* windows
    pub(crate) shared: Rc<SharedContext>,
}

/// A bundle of state shared between all the windows, providing a way for us to communicate with running webview.
pub(crate) struct SharedContext {
    pub(crate) event_handlers: WindowEventHandlers,
    pub(crate) window_close_handlers: WindowCloseHandlers,
    pub(crate) pending_webviews: RefCell<Vec<PendingWebview>>,
    pub(crate) shortcut_manager: ShortcutRegistry,
    pub(crate) proxy: EventLoopProxy<UserWindowEvent>,
    pub(crate) target: EventLoopWindowTarget<UserWindowEvent>,
    pub(crate) websocket: EditWebsocket,
}

impl App {
    pub fn new(mut cfg: Config, virtual_dom: VirtualDom) -> (EventLoop<UserWindowEvent>, Self) {
        let event_loop = cfg
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoopBuilder::<UserWindowEvent>::with_user_event().build());

        let tray_icon_show_window_on_click = cfg.tray_icon_show_window_on_click;

        let app = Self {
            exit_on_last_window_close: cfg.exit_on_last_window_close,
            disable_dma_buf_on_wayland: cfg.disable_dma_buf_on_wayland,
            is_visible_before_start: true,
            webviews: HashMap::new(),
            control_flow: ControlFlow::Wait,
            dom: virtual_dom,
            initial_dom_rebuild_done: false,
            float_all: false,
            show_devtools: false,
            tray_icon_show_window_on_click,
            cfg: Cell::new(Some(cfg)),
            shared: Rc::new(SharedContext {
                event_handlers: WindowEventHandlers::default(),
                window_close_handlers: Default::default(),
                pending_webviews: Default::default(),
                shortcut_manager: ShortcutRegistry::new(),
                proxy: event_loop.create_proxy(),
                target: event_loop.clone(),
                websocket: EditWebsocket::start(),
            }),
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
        self.shared
            .event_handlers
            .apply_event(window_event, &self.shared.target);
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn handle_global_hotkey(&self, event: global_hotkey::GlobalHotKeyEvent) {
        self.shared.shortcut_manager.call_handlers(event);
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
        let proxy = self.shared.proxy.clone();
        dioxus_devtools::connect(move |msg| {
            _ = proxy.send_event(UserWindowEvent::HotReloadEvent(msg));
        })
    }

    pub fn handle_new_window(&mut self) {
        for pending_webview in self.shared.pending_webviews.borrow_mut().drain(..) {
            let app_webview = pending_webview.create_window(&mut self.dom, &self.shared);
            let id = app_webview.desktop_context.window.id();
            self.webviews.insert(id, app_webview);
            _ = self.shared.proxy.send_event(UserWindowEvent::Poll(id));
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

            // If the window is set to close, we can remove it from the list of webviews
            // If the app is set to exit when the last window closes, we should also exit the app
            WindowCloseBehaviour::WindowCloses => {
                #[cfg(debug_assertions)]
                self.persist_window_state();

                self.close_window(id);
            }
        };
    }

    pub fn window_destroyed(&mut self, id: WindowId) {
        self.close_window(id);
    }

    /// Tear down one webview: fire close callbacks, drop the webview (and
    /// with it the target's writer), re-render the DOM, and exit if it was
    /// the last window.
    fn close_window(&mut self, id: WindowId) {
        if !self.webviews.contains_key(&id) {
            return;
        }

        self.shared.window_close_handlers.notify(id);

        // Dropping the webview drops its WryQueue with it; the next render
        // pass simply won't include the target.
        self.webviews.remove(&id);

        self.render_after_webview_removed();

        if self.exit_on_last_window_close && self.webviews.is_empty() {
            self.control_flow = ControlFlow::Exit
        }
    }

    pub fn resize_window(&self, id: WindowId, size: PhysicalSize<u32>) {
        // TODO: the app layer should avoid directly manipulating the webview webview instance internals.
        // Window creation and modification is the responsibility of the webview instance so it makes sense to
        // encapsulate that there.
        if let Some(app_webview) = self.webviews.get(&id) {
            use wry::Rect;

            _ = app_webview
                .desktop_context
                .webview
                .set_bounds(Rect {
                    position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                    size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(
                        size.width,
                        size.height,
                    )),
                });
        }
    }

    pub fn handle_start_cause_init(&mut self) {
        #[allow(unused_mut)]
        let mut cfg = self
            .cfg
            .take()
            .expect("Config should be set before initialization");

        self.is_visible_before_start = cfg.window.window.visible;
        #[cfg(not(target_os = "linux"))]
        {
            cfg.window = cfg.window.with_visible(false);
        }
        let explicit_window_size = cfg.window.window.inner_size;
        let explicit_window_position = cfg.window.window.position;

        let webview = WebviewInstance::new(
            cfg,
            RenderTargetId::ROOT,
            &mut self.dom,
            self.shared.clone(),
        );
        self.provide_root_context(webview.desktop_context.clone());

        // And then attempt to resume from state
        self.resume_from_state(&webview, explicit_window_size, explicit_window_position);

        let id = webview.desktop_context.window.id();
        self.webviews.insert(id, webview);
    }

    fn provide_root_context(&mut self, desktop_context: DesktopContext) {
        let provider: Rc<dyn Document> = Rc::new(DesktopDocument::new(desktop_context.clone()));
        let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
        self.dom.in_scope(ScopeId::ROOT, || {
            provide_context(desktop_context);
            provide_context(provider);
            provide_context(history_provider);
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

    /// The webview is finally loaded
    ///
    /// Let's rebuild it and then start polling it
    pub fn handle_initialize_msg(&mut self, id: WindowId) {
        let Some(target_id) = self.webviews.get(&id).map(|view| view.target_id()) else {
            return;
        };

        if !self.initial_dom_rebuild_done {
            let touched = self.rebuild_dom();
            self.send_edits_to_targets(&touched);
        }

        #[cfg(not(target_os = "linux"))]
        {
            if target_id == RenderTargetId::ROOT {
                if let Some(view) = self.webviews.get(&id) {
                    view.desktop_context
                        .window
                        .set_visible(self.is_visible_before_start);
                }
            }
        }

        _ = self.shared.proxy.send_event(UserWindowEvent::Poll(id));
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
                if let Some(id) = self.webviews.keys().next().copied() {
                    {
                        // This is a place where wry says it's threadsafe but it's actually not.
                        // If we're patching the app, we want to make sure it's not going to progress in the interim.
                        #[cfg(target_os = "android")]
                        let _lock = crate::android_sync_lock::android_runtime_lock();

                        dioxus_devtools::apply_changes(&self.dom, &hr_msg);
                    }

                    self.poll_vdom(id);
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

    /// Poll the virtualdom until it's pending
    ///
    /// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
    ///
    /// All IO is done on the tokio runtime we started earlier
    pub fn poll_vdom(&mut self, id: WindowId) {
        let Some(waker) = self
            .webviews
            .get(&id)
            .map(|app_webview| app_webview.waker.clone())
        else {
            return;
        };
        let mut cx = std::task::Context::from_waker(&waker);

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

            let touched = self.render_dom_immediate();
            self.send_edits_to_targets(&touched);
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
                (
                    app_webview.target_id(),
                    app_webview.edits.wry_queue.clone(),
                )
            })
            .collect()
    }

    /// Collect every webview whose `WryQueue` was touched during the preceding
    /// render pass. The diff writes directly into each registered queue (the
    /// `WriteMutations` impl on `WryQueue`), so a touched queue means "this
    /// webview has new edits to flush".
    fn collect_touched(&self) -> BTreeSet<RenderTargetId> {
        self.webviews
            .values()
            .filter(|app_webview| app_webview.edits.wry_queue.is_touched())
            .map(|app_webview| app_webview.target_id())
            .collect()
    }

    fn rebuild_dom(&mut self) -> BTreeSet<RenderTargetId> {
        let mut writer = self.dom_writer();
        self.dom.rebuild(&mut writer);
        self.initial_dom_rebuild_done = true;
        self.collect_touched()
    }

    fn render_dom_immediate(&mut self) -> BTreeSet<RenderTargetId> {
        let mut writer = self.dom_writer();
        self.dom.render_immediate(&mut writer);
        self.collect_touched()
    }

    fn render_after_webview_removed(&mut self) {
        let touched = self.render_dom_immediate();
        self.send_edits_to_targets(&touched);
        self.poll_next_webview();
    }

    fn send_edits_to_targets(&self, targets: &BTreeSet<RenderTargetId>) {
        for app_webview in self.webviews.values() {
            if targets.contains(&app_webview.target_id()) {
                app_webview.edits.wry_queue.send_edits();
            }
        }
    }

    fn poll_next_webview(&mut self) {
        let next_webview = self.webviews.keys().next().copied();

        if let Some(id) = next_webview {
            self.poll_vdom(id);
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
        let receiver = self.shared.proxy.clone();

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
        let receiver = self.shared.proxy.clone();

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
        let receiver = self.shared.proxy.clone();

        // The event loop becomes the menu receiver
        // This means we don't need to poll the receiver on every tick - we just get the events as they come in
        // This is a bit more efficient than the previous implementation, but if someone else sets a handler, the
        // receiver will become inert.
        tray_icon::TrayIconEvent::set_event_handler(Some(move |t| {
            // todo: should we unset the event handler when the app shuts down?
            _ = receiver.send_event(UserWindowEvent::TrayIconEvent(t));
        }));

        // for whatever reason they had to make it separate
        let receiver = self.shared.proxy.clone();
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

    // Write this to the target dir so we can pick back up
    fn resume_from_state(
        &mut self,
        webview: &WebviewInstance,
        explicit_inner_size: Option<tao::dpi::Size>,
        explicit_window_position: Option<tao::dpi::Position>,
    ) {
        // We only want to do this on desktop
        if cfg!(target_os = "android") || cfg!(target_os = "ios") {
            return;
        }

        // We only want to do this in debug mode
        if !cfg!(debug_assertions) {
            return;
        }

        if let Ok(state) = std::fs::read_to_string(restore_file()) {
            if let Ok(state) = serde_json::from_str::<PreservedWindowState>(&state) {
                let window = &webview.desktop_context.window;
                let position = (state.x, state.y);
                let size = (state.width, state.height);

                // Only set the outer position if it wasn't explicitly set
                if explicit_window_position.is_none() {
                    if cfg!(target_os = "macos") {
                        window.set_outer_position(tao::dpi::LogicalPosition::new(
                            position.0, position.1,
                        ));
                    } else {
                        window.set_outer_position(tao::dpi::PhysicalPosition::new(
                            position.0, position.1,
                        ));
                    }
                }

                // Only set the inner size if it wasn't explicitly set
                if explicit_inner_size.is_none() {
                    if cfg!(target_os = "macos") {
                        window.set_inner_size(tao::dpi::LogicalSize::new(size.0, size.1));
                    } else {
                        window.set_inner_size(tao::dpi::PhysicalSize::new(size.0, size.1));
                    }
                }
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
            let target = self.shared.proxy.clone();
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
struct PreservedWindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor: String,
}

/// Return the location of a tempfile with our window state in it such that we can restore it later
fn restore_file() -> std::path::PathBuf {
    let dir = dioxus_cli_config::session_cache_dir().unwrap_or_else(std::env::temp_dir);
    dir.join("window-state.json")
}
