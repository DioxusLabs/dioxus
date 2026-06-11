use crate::app::MakeVirtualDom;
use crate::desktop_context::DesktopContextInner;
use crate::dom_thread::{DomThreadMessage, VirtualDomEvent};
use crate::menubar::DioxusMenu;
use crate::{
    Config, DesktopService,
    app::SharedContext,
    assets::AssetHandlerRegistry,
    edits::WryQueue,
    file_upload::NativeFileHover,
    ipc::{UserWindowEvent, UserWindowEventVariant, WindowHandle},
    protocol,
};
use dioxus_hooks::to_owned;
use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use tao::{
    event_loop::{EventLoopProxy, EventLoopWindowTarget},
    window::WindowId,
};
use wry::{DragDropEvent, RequestAsyncResponder, WebContext, WebViewBuilder, WebViewId};
use wry_bindgen_runtime::{WryBindgen, WryBindgenWebviewDriver};

pub(crate) struct WebviewInstance {
    /// Sends events to the VirtualDom running on the dedicated DOM thread.
    pub dom_event_tx: tokio::sync::mpsc::UnboundedSender<VirtualDomEvent>,
    pub wry_queue: WryQueue,
    pub desktop_context: Rc<DesktopService>,
    /// Set once this window starts closing (its VirtualDom task is aborted or finished). The
    /// instance stays in the `App::webviews` map until every [`WindowHandle`] drops, so proxied
    /// calls on handles held elsewhere keep working; this flag stops the closing window from
    /// being re-shown (tray click, webview reload) in the meantime.
    pub closing: Cell<bool>,
    wry_bindgen_driver: WryBindgenWebviewDriver,
    wry_bindgen_driver_waker: Waker,
    wry_bindgen_driver_done: bool,

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

struct WryBindgenDriverWake {
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
}

impl Wake for WryBindgenDriverWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self
            .proxy
            .send_event(UserWindowEventVariant::WryBindgenDriverWake(self.window_id).into());
    }
}

fn is_wry_bindgen_request(request: &wry::http::Request<Vec<u8>>) -> bool {
    request
        .uri()
        .path()
        .trim_matches('/')
        .starts_with("__wbg__/")
}

fn wry_bindgen_not_found_response() -> wry::http::Response<Vec<u8>> {
    wry::http::Response::builder()
        .status(wry::http::StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("Failed to build not found response")
}

impl WebviewInstance {
    /// Create a new WebviewInstance.
    ///
    /// The VirtualDom runs on the DOM thread inside this webview's wry-bindgen runtime.
    /// This webview connects to it through the event channel created below.
    /// Returns the instance together with a strong [`WindowHandle`]: the caller either moves it
    /// into a [`DesktopContextInner`] for the window's creator (see
    /// [`PendingWebview::create_window`]) or drops it. The instance itself holds only a weak
    /// reference, so handle holders — not the webviews map — decide how long the window's
    /// main-thread state lives.
    pub(crate) fn new(
        mut cfg: Config,
        dom: MakeVirtualDom,
        shared: Rc<SharedContext>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) -> (WebviewInstance, Arc<WindowHandle>) {
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
            window = window.with_window_icon(crate::default_icon().ok());
        }

        let window = Arc::new(window.build(target).unwrap());

        let proxy = shared.proxy.clone();

        // Create the event channel for VirtualDom communication. The VirtualDom runs on a
        // dedicated thread and receives events from the main thread through this channel.
        let (event_tx, dom_event_rx) = tokio::sync::mpsc::unbounded_channel();

        let wry_bindgen = WryBindgen::new();
        let protocol = wry_bindgen.protocol_handler();

        // Runs on the VirtualDom thread right after the dom is created, before it starts.
        let on_window = cfg.on_window.take();

        // https://developer.apple.com/documentation/appkit/nswindowcollectionbehavior/nswindowcollectionbehaviormanaged
        #[cfg(target_os = "macos")]
        {
            use objc2::rc::Retained;
            use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
            use tao::platform::macos::WindowExtMacOS;
            let ns_window: Retained<NSWindow> =
                unsafe { Retained::retain(window.ns_window().cast()) }.unwrap();
            ns_window.setCollectionBehavior(NSWindowCollectionBehavior::Managed)
        }

        let mut web_context = WebContext::new(cfg.data_dir.clone().or_else(|| {
            // On Windows, WebView2 defaults to storing its data next to the executable.
            // This fails on certain drives (e.g. ReFS dev drives, Program Files) where the
            // directory may not be writable. Fall back to %LOCALAPPDATA%/<exe_name> automatically.
            if cfg!(windows) {
                let exe = std::env::current_exe().ok()?;
                let name = exe.file_stem()?.to_str()?;
                let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
                Some(std::path::PathBuf::from(local_app_data).join(name))
            } else {
                None
            }
        }));
        let edit_queue = shared.websocket.create_queue();
        // The VirtualDom thread sends its rendered edits straight to this webview's websocket.
        let webview_id = edit_queue.webview_id();
        let websocket = shared.websocket.clone();
        let asset_handlers = AssetHandlerRegistry::new();
        let file_hover = NativeFileHover::default();
        let headless = !cfg.window.window.visible;

        let request_handler = {
            to_owned![
                cfg.custom_head,
                cfg.custom_index,
                cfg.root_name,
                asset_handlers,
                edit_queue
            ];

            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            move |_id: WebViewId,
                  request: wry::http::Request<Vec<u8>>,
                  responder: RequestAsyncResponder| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();

                if is_wry_bindgen_request(&request) {
                    let _lock = crate::android_sync_lock::android_runtime_lock();
                    let responder = move |response| responder.respond(response);
                    let Some(responder) = protocol.handle_request("dioxus", &request, responder)
                    else {
                        return;
                    };
                    responder(wry_bindgen_not_found_response());
                    return;
                }

                // Fall through to existing dioxus protocol handler
                protocol::desktop_handler(
                    request,
                    asset_handlers.clone(),
                    responder,
                    &edit_queue,
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
                    _ = proxy.send_event(UserWindowEventVariant::Ipc { id: window_id, msg }.into());
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
                            _ = proxy.send_event(
                                UserWindowEventVariant::WindowsDragDrop(window_id).into(),
                            );
                        }
                        wry::DragDropEvent::Over { position } => {
                            _ = proxy.send_event(
                                UserWindowEventVariant::WindowsDragOver(
                                    window_id, position.0, position.1,
                                )
                                .into(),
                            );
                        }
                        wry::DragDropEvent::Leave => {
                            _ = proxy.send_event(
                                UserWindowEventVariant::WindowsDragLeave(window_id).into(),
                            );
                        }
                        _ => {}
                    }
                }

                false
            }
        };

        let navigation_handler = cfg.navigation_handler.take();
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
                    return !page_loaded;
                }

                // External links always open somewhere else. Prevents the webview from navigating
                if var.starts_with("http://")
                    || var.starts_with("https://")
                    || var.starts_with("mailto:")
                {
                    _ = webbrowser::open(&var);
                    return false;
                }

                // By default, external links are allowed. This keeps things like iframes working.
                // However, users can customize this to allow/disallow domains/routes/patterns.
                navigation_handler.as_ref().map(|f| f(&var)).unwrap_or(true)
            })
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

        let window_id = window.id();
        let window_handle = Arc::new(WindowHandle {
            proxy: proxy.clone(),
            window_id,
        });

        let desktop_context = Rc::from(DesktopService::new(
            webview,
            window,
            shared.clone(),
            asset_handlers,
            cfg.window_close_behavior,
            event_tx.clone(),
            Arc::downgrade(&window_handle),
        ));

        // Finally spawn the app in the virtual dom task thread
        let run_app = {
            // Captured in the closure environment (not just the async body) so the handle is
            // released even if the app future is dropped before the webview ever invokes it.
            let window_handle = window_handle.clone();
            let event_tx = event_tx.clone();
            let window = desktop_context.window.clone();
            move || async move {
                let mut dom = dom();

                if let Some(mut on_window) = on_window {
                    on_window(window, &mut dom);
                }

                crate::dom_thread::run_virtual_dom_with_dom(
                    dom,
                    dom_event_rx,
                    event_tx,
                    websocket,
                    webview_id,
                    window_handle,
                    file_hover,
                )
                .await;
            }
        };
        let (runtime, driver) = wry_bindgen.split();
        let future = runtime.run(run_app);
        _ = shared
            .desktop_thread_handle
            .tx
            .send(DomThreadMessage::Spawn(
                window_id,
                Box::new(|| Box::pin(future.into_future())),
            ));

        // Request an initial redraw
        desktop_context.window.request_redraw();

        let wry_bindgen_driver = driver.with_evaluate_script({
            let desktop_context = desktop_context.clone();
            move |script| {
                let _ = desktop_context.webview.evaluate_script(script);
            }
        });
        let wry_bindgen_driver_waker = Waker::from(Arc::new(WryBindgenDriverWake {
            proxy: shared.proxy.clone(),
            window_id,
        }));

        let mut instance = WebviewInstance {
            dom_event_tx: event_tx,
            wry_queue: edit_queue,
            desktop_context,
            closing: Cell::new(false),
            wry_bindgen_driver,
            wry_bindgen_driver_waker,
            wry_bindgen_driver_done: false,
            _menu: menu,
            _web_context: web_context,
        };

        // Prime the wry-bindgen driver so it registers its waker; afterwards it is only polled
        // when that waker fires (via `WryBindgenDriverWake`).
        instance.poll_wry_bindgen_driver();

        (instance, window_handle)
    }

    pub(crate) fn poll_wry_bindgen_driver(&mut self) {
        if self.wry_bindgen_driver_done {
            return;
        }

        let mut cx = Context::from_waker(&self.wry_bindgen_driver_waker);
        if matches!(self.wry_bindgen_driver.poll(&mut cx), Poll::Ready(())) {
            self.wry_bindgen_driver_done = true;
        }
    }

    /// Re-point the webview's interpreter at the edit websocket's current location.
    ///
    /// The socket may be killed by the OS while running. On iOS the websocket is killed when
    /// the device goes to sleep; when that happens the server rebinds to a new port and key
    /// and we tell the webview to reconnect to the new location so it keeps receiving edits.
    /// <https://github.com/DioxusLabs/dioxus/issues/4374>
    pub fn send_edits_location(&self) {
        _ = self.desktop_context.webview.evaluate_script(&format!(
            "window.interpreter.waitForRequest(\"{edits_path}\", \"{expected_key}\");",
            edits_path = self.wry_queue.edits_path(),
            expected_key = self.wry_queue.required_server_key()
        ));
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
/// Each queued webview gets its own wry-bindgen runtime and driver when created.
pub(crate) struct PendingWebview {
    dom: MakeVirtualDom,
    cfg: Config,
    sender: futures_channel::oneshot::Sender<DesktopContextInner>,
}

impl PendingWebview {
    pub(crate) fn new(
        cfg: Config,
        dom: MakeVirtualDom,
        sender: futures_channel::oneshot::Sender<DesktopContextInner>,
    ) -> Self {
        Self { cfg, sender, dom }
    }

    pub(crate) fn create_window(
        self,
        shared: &Rc<SharedContext>,
        target: &EventLoopWindowTarget<UserWindowEvent>,
    ) -> WebviewInstance {
        let (window, window_handle) =
            WebviewInstance::new(self.cfg, self.dom, shared.clone(), target);

        // Return the desktop service proxy to the pending future. The strong window handle moves
        // into it: the resolved DesktopContext keeps the window's main-thread state alive.
        _ = self
            .sender
            .send(window.desktop_context.proxy_inner(window_handle));

        window
    }
}
