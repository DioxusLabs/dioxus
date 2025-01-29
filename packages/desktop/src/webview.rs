use crate::element::DesktopElement;
use crate::file_upload::DesktopFileDragEvent;
use crate::menubar::DioxusMenu;
use crate::{
    app::SharedContext,
    assets::AssetHandlerRegistry,
    edits::WryQueue,
    file_upload::{NativeFileEngine, NativeFileHover},
    ipc::UserWindowEvent,
    protocol,
    waker::tao_waker,
    Config, DesktopContext, DesktopService,
};
use crate::{document::DesktopDocument, WeakDesktopContext};
use base64::prelude::BASE64_STANDARD;
use dioxus_core::{Runtime, ScopeId, VirtualDom};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use dioxus_hooks::to_owned;
use dioxus_html::{HasFileData, HtmlEvent, PlatformEventData};
use futures_util::{pin_mut, FutureExt};
use std::cell::OnceCell;
use std::sync::Arc;
use std::{rc::Rc, task::Waker};
use wry::{DragDropEvent, RequestAsyncResponder, WebContext, WebViewBuilder};

#[derive(Clone)]
pub(crate) struct WebviewEdits {
    runtime: Rc<Runtime>,
    pub wry_queue: WryQueue,
    desktop_context: Rc<OnceCell<WeakDesktopContext>>,
}

impl WebviewEdits {
    fn new(runtime: Rc<Runtime>, wry_queue: WryQueue) -> Self {
        Self {
            runtime,
            wry_queue,
            desktop_context: Default::default(),
        }
    }

    fn set_desktop_context(&self, context: WeakDesktopContext) {
        _ = self.desktop_context.set(context);
    }

    pub fn handle_event(
        &self,
        request: wry::http::Request<Vec<u8>>,
        responder: wry::RequestAsyncResponder,
    ) {
        let body = self
            .try_handle_event(request)
            .expect("Writing bodies to succeed");
        responder.respond(wry::http::Response::new(body))
    }

    pub fn try_handle_event(
        &self,
        request: wry::http::Request<Vec<u8>>,
    ) -> Result<Vec<u8>, serde_json::Error> {
        use serde::de::Error;

        // todo(jon):
        //
        // I'm a small bit worried about the size of the header being too big on some platforms.
        // It's unlikely we'll hit the 256k limit (from 2010 browsers...) but it's important to think about
        // https://stackoverflow.com/questions/3326210/can-http-headers-be-too-big-for-browsers
        //
        // Also important to remember here that we don't pass a body from the JavaScript side of things
        let data = request
            .headers()
            .get("dioxus-data")
            .ok_or_else(|| Error::custom("dioxus-data header not set"))?;

        let as_utf = std::str::from_utf8(data.as_bytes())
            .map_err(|_| Error::custom("dioxus-data header is not a valid (utf-8) string"))?;

        let data_from_header = base64::Engine::decode(&BASE64_STANDARD, as_utf)
            .map_err(|_| Error::custom("dioxus-data header is not a base64 string"))?;

        let response = match serde_json::from_slice(&data_from_header) {
            Ok(event) => {
                // we need to wait for the mutex lock to let us munge the main thread..
                let _lock = crate::android_sync_lock::android_runtime_lock();
                self.handle_html_event(event)
            }
            Err(err) => {
                tracing::error!(
                    "Error parsing user_event: {:?}.Contents: {:?}, raw: {:#?}",
                    err,
                    String::from_utf8(request.body().to_vec()),
                    request
                );
                SynchronousEventResponse::new(false)
            }
        };

        serde_json::to_vec(&response).inspect_err(|err| {
            tracing::error!("failed to serialize SynchronousEventResponse: {err:?}");
        })
    }

    pub fn handle_html_event(&self, event: HtmlEvent) -> SynchronousEventResponse {
        let HtmlEvent {
            element,
            name,
            bubbles,
            data,
        } = event;
        let Some(desktop_context) = self.desktop_context.get() else {
            tracing::error!(
                "Tried to handle event before setting the desktop context on the event handler"
            );
            return Default::default();
        };

        let desktop_context = desktop_context.upgrade().unwrap();

        let query = desktop_context.query.clone();
        let recent_file = desktop_context.file_hover.clone();

        // check for a mounted event placeholder and replace it with a desktop specific element
        let as_any = match data {
            dioxus_html::EventData::Mounted => {
                let element = DesktopElement::new(element, desktop_context.clone(), query.clone());
                Rc::new(PlatformEventData::new(Box::new(element)))
            }
            dioxus_html::EventData::Drag(ref drag) => {
                // we want to override this with a native file engine, provided by the most recent drag event
                if drag.files().is_some() {
                    let file_event = recent_file.current().unwrap();
                    let paths = match file_event {
                        wry::DragDropEvent::Enter { paths, .. } => paths,
                        wry::DragDropEvent::Drop { paths, .. } => paths,
                        _ => vec![],
                    };
                    Rc::new(PlatformEventData::new(Box::new(DesktopFileDragEvent {
                        mouse: drag.mouse.clone(),
                        files: Arc::new(NativeFileEngine::new(paths)),
                    })))
                } else {
                    data.into_any()
                }
            }
            _ => data.into_any(),
        };

        let event = dioxus_core::Event::new(as_any, bubbles);
        self.runtime.handle_event(&name, event.clone(), element);

        // Get the response from the event
        SynchronousEventResponse::new(!event.default_action_enabled())
    }
}

pub(crate) struct WebviewInstance {
    pub dom: VirtualDom,
    pub edits: WebviewEdits,
    pub desktop_context: DesktopContext,
    pub waker: Waker,

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
    pub(crate) fn new(
        mut cfg: Config,
        dom: VirtualDom,
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

        let window = window.build(&shared.target).unwrap();

        // https://developer.apple.com/documentation/appkit/nswindowcollectionbehavior/nswindowcollectionbehaviormanaged
        #[cfg(target_os = "macos")]
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
        let edit_queue = WryQueue::default();
        let asset_handlers = AssetHandlerRegistry::new();
        let edits = WebviewEdits::new(dom.runtime(), edit_queue.clone());
        let file_hover = NativeFileHover::default();
        let headless = !cfg.window.window.visible;

        let request_handler = {
            to_owned![
                cfg.custom_head,
                cfg.custom_index,
                cfg.root_name,
                asset_handlers,
                edits
            ];
            move |request, responder: RequestAsyncResponder| {
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
                // defer the event to the main thread
                let body = payload.into_body();
                if let Ok(msg) = serde_json::from_str(&body) {
                    _ = proxy.send_event(UserWindowEvent::Ipc { id: window_id, msg });
                }
            }
        };

        let file_drop_handler = {
            to_owned![file_hover];

            #[cfg(windows)]
            let (proxy, window_id) = (shared.proxy.to_owned(), window.id());

            move |evt: DragDropEvent| {
                // Update the most recent file drop event - when the event comes in from the webview we can use the
                // most recent event to build a new event with the files in it.
                #[cfg(not(windows))]
                file_hover.set(evt);

                // Windows webview blocks HTML-native events when the drop handler is provided.
                // The problem is that the HTML-native events don't provide the file, so we need this.
                // Solution: this glue code to mimic drag drop events.
                #[cfg(windows)]
                {
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

        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        let mut webview = if cfg.as_child_window {
            WebViewBuilder::new_as_child(&window)
        } else {
            WebViewBuilder::new(&window)
        };

        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        )))]
        let mut webview = {
            use tao::platform::unix::WindowExtUnix;
            use wry::WebViewBuilderExtUnix;
            let vbox = window.default_vbox().unwrap();
            WebViewBuilder::new_gtk(vbox)
        };

        // Disable the webview default shortcuts to disable the reload shortcut
        #[cfg(target_os = "windows")]
        {
            use wry::WebViewBuilderExtWindows;
            webview = webview.with_browser_accelerator_keys(false);
        }

        webview = webview
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
            .with_navigation_handler(|var| {
                // We don't want to allow any navigation
                // We only want to serve the index file and assets
                if var.starts_with("dioxus://") || var.starts_with("http://dioxus.") {
                    true
                } else {
                    if var.starts_with("http://") || var.starts_with("https://") {
                        _ = webbrowser::open(&var);
                    }
                    false
                }
            }) // prevent all navigations
            .with_asynchronous_custom_protocol(String::from("dioxus"), request_handler)
            .with_web_context(&mut web_context)
            .with_drag_drop_handler(file_drop_handler);

        if let Some(color) = cfg.background_color {
            webview = webview.with_background_color(color);
        }

        for (name, handler) in cfg.protocols.drain(..) {
            webview = webview.with_custom_protocol(name, handler);
        }

        for (name, handler) in cfg.asynchronous_protocols.drain(..) {
            webview = webview.with_asynchronous_custom_protocol(name, handler);
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

        let webview = webview.build().unwrap();

        let menu = if cfg!(not(any(target_os = "android", target_os = "ios"))) {
            let menu_option = cfg.menu.into();
            if let Some(menu) = &menu_option {
                crate::menubar::init_menu_bar(menu, &window);
            }
            menu_option
        } else {
            None
        };

        let desktop_context = Rc::from(DesktopService::new(
            webview,
            window,
            shared.clone(),
            asset_handlers,
            file_hover,
        ));

        // Provide the desktop context to the virtual dom and edit handler
        edits.set_desktop_context(Rc::downgrade(&desktop_context));
        let provider: Rc<dyn Document> = Rc::new(DesktopDocument::new(desktop_context.clone()));
        let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
        dom.in_runtime(|| {
            ScopeId::ROOT.provide_context(desktop_context.clone());
            ScopeId::ROOT.provide_context(provider);
            ScopeId::ROOT.provide_context(history_provider);
        });

        WebviewInstance {
            dom,
            edits,
            waker: tao_waker(shared.proxy.clone(), desktop_context.window.id()),
            desktop_context,
            _menu: menu,
            _web_context: web_context,
        }
    }

    pub fn poll_vdom(&mut self) {
        let mut cx = std::task::Context::from_waker(&self.waker);

        // Continuously poll the virtualdom until it's pending
        // Wait for work will return Ready when it has edits to be sent to the webview
        // It will return Pending when it needs to be polled again - nothing is ready
        loop {
            // If we're waiting for a render, wait for it to finish before we continue
            let edits_flushed_poll = self.edits.wry_queue.poll_edits_flushed(&mut cx);
            if edits_flushed_poll.is_pending() {
                return;
            }

            {
                // lock the hack-ed in lock sync wry has some thread-safety issues with event handlers and async tasks
                let _lock = crate::android_sync_lock::android_runtime_lock();
                let fut = self.dom.wait_for_work();
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => return,
                }
            }

            // lock the hack-ed in lock sync wry has some thread-safety issues with event handlers
            let _lock = crate::android_sync_lock::android_runtime_lock();

            self.edits
                .wry_queue
                .with_mutation_state_mut(|f| self.dom.render_immediate(f));
            self.edits.wry_queue.send_edits();
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
}

/// A synchronous response to a browser event which may prevent the default browser's action
#[derive(serde::Serialize, Default)]
pub struct SynchronousEventResponse {
    #[serde(rename = "preventDefault")]
    prevent_default: bool,
}

impl SynchronousEventResponse {
    /// Create a new SynchronousEventResponse
    #[allow(unused)]
    pub fn new(prevent_default: bool) -> Self {
        Self { prevent_default }
    }
}
