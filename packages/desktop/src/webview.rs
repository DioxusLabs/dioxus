use crate::app::MakeVirtualDom;
use crate::dom_thread::{MainThreadCommand, VirtualDomEvent, VirtualDomHandle};
use crate::menubar::DioxusMenu;
use crate::PendingDesktopContext;
use crate::{
    app::SharedContext, assets::AssetHandlerRegistry, edits::WryQueue,
    file_upload::NativeFileHover, ipc::UserWindowEvent, protocol, Config, DesktopContext,
    DesktopService,
};
use dioxus_hooks::to_owned;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Waker;
use std::time::Duration;
use wry::{DragDropEvent, RequestAsyncResponder, WebContext, WebViewBuilder, WebViewId};
use wry_bindgen::wry::{ImplWryBindgenResponder, WryBindgenResponder};

/// This struct manages the webview's communication with the VirtualDom thread.
///
/// Events are now handled via wasm-bindgen closure (see wry_bindgen_bridge.rs),
/// not through this struct. This struct primarily manages the WryQueue for mutations.
#[derive(Clone)]
pub(crate) struct WebviewEdits {
    pub wry_queue: WryQueue,
}

impl WebviewEdits {
    fn new(wry_queue: WryQueue) -> Self {
        Self { wry_queue }
    }
}

pub(crate) struct WebviewInstance {
    /// Handle to communicate with the VirtualDom running on a dedicated thread.
    pub dom_handle: VirtualDomHandle,
    pub edits: WebviewEdits,
    pub desktop_context: Rc<DesktopService>,

    /// Channel to receive commands from the VirtualDom.
    /// Uses futures channel for proper async polling with wakers.
    pub(crate) dom_command_rx: futures_channel::mpsc::UnboundedReceiver<MainThreadCommand>,

    /// Waker that sends Poll events to the event loop when async work completes.
    waker: Waker,

    // Wry assumes the webcontext is alive for the lifetime of the webview.
    // We need to keep the webcontext alive, otherwise the webview will crash
    _web_context: WebContext,

    // Same with the menu.
    // Currently it's a DioxusMenu because 1) we don't touch it and 2) we support a number of platforms
    // like ios where muda does not give us a menu type. It sucks but alas.
    //
    // This would be a good thing for someone looking to contribute to fix.
    _menu: Option<DioxusMenu>,
}

impl WebviewInstance {
    /// Create a new WebviewInstance.
    ///
    /// The VirtualDom is already running in the wry-bindgen thread (started in App::new).
    /// This webview connects to it via the shared channels in SharedContext.
    pub(crate) fn new(
        mut cfg: Config,
        make_dom: MakeVirtualDom,
        shared: Rc<SharedContext>,
    ) -> WebviewInstance {
        let mut window = cfg.window.clone();

        // tao makes small windows for some reason, make them bigger on desktop
        //
        // on mobile, we want them to be `None` so tao makes them the size of the screen. Otherwise we
        // get a window that is not the size of the screen and weird black bars.
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            if cfg.window.window.inner_size.is_none() {
                window = window.with_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0));
            }
        }

        // We assume that if the icon is None in cfg, then the user just didnt set it
        if cfg.window.window.window_icon.is_none() {
            window = window.with_window_icon(Some(
                tao::window::Icon::from_rgba(
                    include_bytes!("./assets/default_icon.bin").to_vec(),
                    460,
                    460,
                )
                .expect("image parse failed"),
            ));
        }

        let window = Arc::new(window.build(&shared.target).unwrap());

        let proxy = shared.proxy.clone();

        // Create channels for VirtualDom communication
        // The VirtualDom runs in the wry-bindgen thread, communicating via these channels
        let (dom_event_tx, dom_event_rx) = tokio::sync::mpsc::unbounded_channel();
        // Use futures channel for proper async polling with wakers on the main thread
        let (dom_command_tx, dom_command_rx) = futures_channel::mpsc::unbounded();

        // Start building the wry bindgen future
        let app_builder = shared.wry_bindgen.app_builder();
        // Get the wry bindgen protocol handler
        let protocol = app_builder.protocol_handler();

        // TODO: restore on dom thread or remove dom access
        // if let Some(on_build) = cfg.on_window.as_mut() {
        //     on_build(window.clone(), &mut dom);
        // }

        // https://developer.apple.com/documentation/appkit/nswindowcollectionbehavior/nswindowcollectionbehaviormanaged
        #[cfg(target_os = "macos")]
        #[allow(deprecated)]
        {
            use cocoa::appkit::NSWindowCollectionBehavior;
            use cocoa::base::id;
            use objc::{msg_send, sel, sel_impl};
            use tao::platform::macos::WindowExtMacOS;

            unsafe {
                let window: id = window.ns_window() as id;
                #[allow(unexpected_cfgs)]
                let _: () = msg_send![window, setCollectionBehavior: NSWindowCollectionBehavior::NSWindowCollectionBehaviorManaged];
            }
        }

        let mut web_context = WebContext::new(cfg.data_dir.clone());
        let edit_queue = shared.websocket.create_queue();
        let asset_handlers = AssetHandlerRegistry::new();
        let file_hover = NativeFileHover::default();
        let headless = !cfg.window.window.visible;

        // Use shared channels for VirtualDom communication
        // The VirtualDom is already running in the wry-bindgen thread
        let event_tx = dom_event_tx;

        let edits = WebviewEdits::new(edit_queue.clone());

        let request_handler = {
            to_owned![
                cfg.custom_head,
                cfg.custom_index,
                cfg.root_name,
                asset_handlers,
                edits,
                shared
            ];

            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            move |_id: WebViewId,
                  request: wry::http::Request<Vec<u8>>,
                  responder: RequestAsyncResponder| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();
                let _lock = crate::android_sync_lock::android_runtime_lock();

                struct ResponderWrapper {
                    responder: RequestAsyncResponder,
                }

                impl Into<WryBindgenResponder> for ResponderWrapper {
                    fn into(self) -> WryBindgenResponder {
                        WryBindgenResponder::new(self)
                    }
                }

                impl ImplWryBindgenResponder for ResponderWrapper {
                    fn respond(self: Box<Self>, response: wry::http::Response<Vec<u8>>) {
                        self.responder.respond(response);
                    }
                }

                let responder = ResponderWrapper { responder };
                // Create wry-bindgen protocol handler wrapped in Rc for sharing
                let protocol_name = "dioxus";
                let proxy = shared.proxy.clone();
                let send_app_event = move |event| {
                    let _ = proxy.send_event(UserWindowEvent::WryBindgenEvent(event));
                };
                let response =
                    protocol.handle_request(protocol_name, send_app_event, &request, responder);
                let Some(responder) = response else {
                    return;
                };
                let responder = responder.responder;

                // Fall through to existing dioxus protocol handler
                protocol::desktop_handler(
                    request,
                    asset_handlers.clone(),
                    responder,
                    &edits,
                    custom_head.clone(),
                    custom_index.clone(),
                    &root_name,
                    headless,
                )
            }
        };

        let ipc_handler = {
            let window_id = window.id();
            to_owned![shared.proxy];
            move |payload: wry::http::Request<String>| {
                let _guard = crate::android_sync_lock::android_runtime_lock();
                // defer the event to the main thread
                let body = payload.into_body();
                if let Ok(msg) = serde_json::from_str(&body) {
                    _ = proxy.send_event(UserWindowEvent::Ipc { id: window_id, msg });
                }
            }
        };

        let file_drop_handler = {
            to_owned![file_hover];
            let (proxy, window_id) = (shared.proxy.to_owned(), window.id());
            move |evt: DragDropEvent| {
                let _guard = crate::android_sync_lock::android_runtime_lock();
                if cfg!(not(windows)) {
                    // Update the most recent file drop event - when the event comes in from the webview we can use the
                    // most recent event to build a new event with the files in it.
                    file_hover.set(evt);
                } else {
                    // Windows webview blocks HTML-native events when the drop handler is provided.
                    // The problem is that the HTML-native events don't provide the file, so we need this.
                    // Solution: this glue code to mimic drag drop events.
                    file_hover.set(evt.clone());
                    match evt {
                        wry::DragDropEvent::Drop {
                            paths: _,
                            position: _,
                        } => {
                            _ = proxy.send_event(UserWindowEvent::WindowsDragDrop(window_id));
                        }
                        wry::DragDropEvent::Over { position } => {
                            _ = proxy.send_event(UserWindowEvent::WindowsDragOver(
                                window_id, position.0, position.1,
                            ));
                        }
                        wry::DragDropEvent::Leave => {
                            _ = proxy.send_event(UserWindowEvent::WindowsDragLeave(window_id));
                        }
                        _ => {}
                    }
                }

                false
            }
        };

        let page_loaded = std::sync::atomic::AtomicBool::new(false);

        let mut webview = WebViewBuilder::new_with_web_context(&mut web_context)
            .with_bounds(wry::Rect {
                position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                size: wry::dpi::Size::Physical(wry::dpi::PhysicalSize::new(
                    window.inner_size().width,
                    window.inner_size().height,
                )),
            })
            .with_transparent(cfg.window.window.transparent)
            .with_url("dioxus://index.html/")
            .with_ipc_handler(ipc_handler)
            .with_navigation_handler(move |var| {
                let _guard = crate::android_sync_lock::android_runtime_lock();
                // We don't want to allow any navigation
                // We only want to serve the index file and assets
                if var.starts_with("dioxus://")
                    || var.starts_with("http://dioxus.")
                    || var.starts_with("https://dioxus.")
                {
                    // After the page has loaded once, don't allow any more navigation
                    let page_loaded = page_loaded.swap(true, std::sync::atomic::Ordering::SeqCst);
                    !page_loaded
                } else {
                    if var.starts_with("http://")
                        || var.starts_with("https://")
                        || var.starts_with("mailto:")
                    {
                        _ = webbrowser::open(&var);
                    }
                    false
                }
            }) // prevent all navigations
            .with_asynchronous_custom_protocol(String::from("dioxus"), request_handler);

        // Enable https scheme on android, needed for secure context API, like the geolocation API
        #[cfg(target_os = "android")]
        {
            use wry::WebViewBuilderExtAndroid as _;

            webview = webview.with_https_scheme(true);
        };

        // Disable the webview default shortcuts to disable the reload shortcut
        #[cfg(target_os = "windows")]
        {
            use wry::WebViewBuilderExtWindows;
            webview = webview.with_browser_accelerator_keys(false);
        }

        if !cfg.disable_file_drop_handler {
            webview = webview.with_drag_drop_handler(file_drop_handler);
        }

        if let Some(color) = cfg.background_color {
            webview = webview.with_background_color(color);
        }

        for (name, handler) in cfg.protocols.drain(..) {
            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            webview = webview.with_custom_protocol(name, move |a, b| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();
                let _lock = crate::android_sync_lock::android_runtime_lock();
                handler(a, b)
            });
        }

        for (name, handler) in cfg.asynchronous_protocols.drain(..) {
            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            webview = webview.with_asynchronous_custom_protocol(name, move |a, b, c| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();
                let _lock = crate::android_sync_lock::android_runtime_lock();
                handler(a, b, c)
            });
        }

        const INITIALIZATION_SCRIPT: &str = r#"
        if (document.addEventListener) {
            document.addEventListener('contextmenu', function(e) {
                e.preventDefault();
            }, false);
        } else {
            document.attachEvent('oncontextmenu', function() {
                window.event.returnValue = false;
            });
        }
        "#;

        if cfg.disable_context_menu {
            // in release mode, we don't want to show the dev tool or reload menus
            webview = webview.with_initialization_script(INITIALIZATION_SCRIPT)
        } else {
            // in debug, we are okay with the reload menu showing and dev tool
            webview = webview.with_devtools(true);
        }

        let menu = if cfg!(not(any(target_os = "android", target_os = "ios"))) {
            let menu_option = cfg.menu.into();
            if let Some(menu) = &menu_option {
                crate::menubar::init_menu_bar(menu, &window);
            }
            menu_option
        } else {
            None
        };

        #[cfg(target_os = "windows")]
        {
            use wry::WebViewBuilderExtWindows;
            if let Some(additional_windows_args) = &cfg.additional_windows_args {
                webview = webview.with_additional_browser_args(additional_windows_args);
            }
        }

        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        let webview = if cfg.as_child_window {
            webview.build_as_child(&window)
        } else {
            webview.build(&window)
        };

        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        let webview = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().unwrap();
            webview.build_gtk(vbox)
        };
        let webview = webview.unwrap();

        let desktop_context = Rc::from(DesktopService::new(
            webview,
            window,
            shared.clone(),
            asset_handlers,
            cfg.window_close_behavior,
            event_tx.clone(),
        ));

        // Finally spawn the app in the virtual dom task thread
        let window_id = desktop_context.window.id();
        let run_app = {
            let proxy = proxy.clone();
            let event_tx = event_tx.clone();
            move || {
                crate::dom_thread::run_virtual_dom(
                    make_dom,
                    dom_event_rx,
                    event_tx,
                    dom_command_tx,
                    proxy,
                    window_id,
                    file_hover,
                )
            }
        };
        let evaluate_script = {
            let desktop_context = desktop_context.clone();
            move |script: &str| {
                // Evaluate script in the webview
                let _ = desktop_context.webview.evaluate_script(script);
            }
        };
        let future = app_builder.build(run_app, evaluate_script);
        _ = shared
            .desktop_thread_handle
            .task_tx
            .send((window_id, Box::new(|| future.into_future())));

        // Create a handle to communicate with the shared VirtualDom
        // The VirtualDom is already running in the wry-bindgen thread (started in App::new)
        // Commands are received via App::process_dom_commands from the shared channel
        let dom_handle = VirtualDomHandle::new(event_tx);

        // Request an initial redraw
        desktop_context.window.request_redraw();

        // Create a waker that sends Poll events to the event loop
        let waker = crate::waker::tao_waker(shared.proxy.clone(), desktop_context.window.id());

        WebviewInstance {
            dom_handle,
            edits,
            desktop_context,
            dom_command_rx,
            waker,
            _menu: menu,
            _web_context: web_context,
        }
    }

    /// Send raw mutation bytes to the webview via websocket.
    pub fn send_edits_raw(&mut self, edits: Vec<u8>) {
        self.edits.wry_queue.send_edits_raw(edits);
    }

    /// Check if pending edits have been acknowledged by the webview.
    /// Returns true if edits were flushed (and sends EditsAcknowledged to VirtualDom).
    pub fn poll_edits_flushed(&mut self) -> bool {
        // Use the stored waker which will send Poll events to wake up the event loop
        let mut cx = std::task::Context::from_waker(&self.waker);

        if self.edits.wry_queue.poll_edits_flushed(&mut cx).is_ready() {
            self.dom_handle
                .send_event(VirtualDomEvent::EditsAcknowledged);
            true
        } else {
            false
        }
    }

    /// Poll for and process commands from the VirtualDom thread.
    ///
    /// Uses the webview's waker to register for wake-up when commands arrive.
    /// This ensures the event loop is woken even if called before commands are ready.
    pub fn poll_dom_commands(&mut self) {
        use crate::dom_thread::MainThreadCommand;
        use futures_util::StreamExt;
        use std::task::Poll;

        // Collect commands first to avoid borrow conflicts
        let commands: Vec<MainThreadCommand> = {
            let mut cx = std::task::Context::from_waker(&self.waker);
            let rx = &mut self.dom_command_rx;
            let mut commands = Vec::new();

            loop {
                match rx.poll_next_unpin(&mut cx) {
                    Poll::Ready(Some(cmd)) => {
                        commands.push(cmd);
                    }
                    Poll::Ready(None) | Poll::Pending => {
                        break;
                    }
                }
            }
            commands
        };

        // Process collected commands
        for cmd in commands {
            match cmd {
                MainThreadCommand::Mutations(edits) => {
                    self.send_edits_raw(edits);
                }
            }
        }
    }

    /// Poll for and process commands from the VirtualDom thread.
    ///
    /// Uses the webview's waker to register for wake-up when commands arrive.
    /// This ensures the event loop is woken even if called before commands are ready.
    pub fn poll_new_edits_location(&mut self) {
        let mut cx = std::task::Context::from_waker(&self.waker);
        // Check if there is a new edit channel we need to send. On IOS,
        // the websocket will be killed when the device is put into sleep. If we
        // find the socket has been closed, we create a new socket and send it to
        // the webview to continue on
        // https://github.com/DioxusLabs/dioxus/issues/4374
        if self
            .edits
            .wry_queue
            .poll_new_edits_location(&mut cx)
            .is_ready()
        {
            _ = self.desktop_context.webview.evaluate_script(&format!(
                "window.interpreter.waitForRequest(\"{edits_path}\", \"{expected_key}\");",
                edits_path = self.edits.wry_queue.edits_path(),
                expected_key = self.edits.wry_queue.required_server_key()
            ));
        }
    }

    #[cfg(all(feature = "devtools", debug_assertions))]
    pub fn kick_stylsheets(&self) {
        // run eval in the webview to kick the stylesheets by appending a query string
        // we should do something less clunky than this
        _ = self
            .desktop_context
            .webview
            .evaluate_script("window.interpreter.kickAllStylesheetsOnPage()");
    }

    /// Displays a toast to the developer.
    pub(crate) fn show_toast(
        &self,
        header_text: &str,
        message: &str,
        level: &str,
        duration: Duration,
        after_reload: bool,
    ) {
        let as_ms = duration.as_millis();

        let js_fn_name = match after_reload {
            true => "scheduleDXToast",
            false => "showDXToast",
        };

        _ = self.desktop_context.webview.evaluate_script(&format!(
            r#"
                if (typeof {js_fn_name} !== "undefined") {{
                    window.{js_fn_name}("{header_text}", "{message}", "{level}", {as_ms});
                }}
                "#,
        ));
    }
}

/// A webview that is queued to be created. We can't spawn webviews outside of the main event loop because it may
/// block on windows so we queue them into the shared context and then create them when the main event loop is ready.
///
/// Note: With wry-bindgen integration, all webviews share the same VirtualDom running in the wry-bindgen thread.
pub(crate) struct PendingWebview {
    dom: MakeVirtualDom,
    cfg: Config,
    sender: futures_channel::oneshot::Sender<DesktopContext>,
}

impl PendingWebview {
    pub(crate) fn new(cfg: Config, dom: MakeVirtualDom) -> (Self, PendingDesktopContext) {
        let (sender, receiver) = futures_channel::oneshot::channel();
        let webview = Self { cfg, sender, dom };
        let pending = PendingDesktopContext { receiver };
        (webview, pending)
    }

    pub(crate) fn create_window(self, shared: &Rc<SharedContext>) -> WebviewInstance {
        let window = WebviewInstance::new(self.cfg, self.dom, shared.clone());

        // Return the desktop service proxy to the pending future
        _ = self.sender.send(window.desktop_context.proxy());

        window
    }
}
