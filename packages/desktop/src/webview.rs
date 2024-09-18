use crate::element::DesktopElement;
use crate::file_upload::DesktopFileDragEvent;
use crate::file_upload::NativeFileEngine;
use crate::menubar::DioxusMenu;
use crate::{
    app::SharedContext, assets::AssetHandlers, edits::WryQueue, file_upload::NativeFileHover,
    ipc::UserWindowEvent, protocol, waker::tao_waker, Config, DesktopContext, DesktopService,
};
use dioxus_core::{Runtime, ScopeId, VirtualDom};
use dioxus_document::Document;
use dioxus_hooks::to_owned;
use dioxus_html::{HasFileData, HtmlEvent, PlatformEventData};
use futures_util::{pin_mut, FutureExt};
use std::cell::OnceCell;
use std::sync::Arc;
use std::{rc::Rc, task::Waker};
use wry::{RequestAsyncResponder, WebContext, WebViewBuilder};

#[derive(Clone)]
pub(crate) struct WebviewEdits {
    runtime: Rc<Runtime>,
    pub wry_queue: WryQueue,
    desktop_context: Rc<OnceCell<DesktopContext>>,
}

impl WebviewEdits {
    fn new(runtime: Rc<Runtime>, wry_queue: WryQueue) -> Self {
        Self {
            runtime,
            wry_queue,
            desktop_context: Default::default(),
        }
    }

    fn set_desktop_context(&self, context: DesktopContext) {
        _ = self.desktop_context.set(context);
    }

    pub fn handle_event(
        &self,
        request: wry::http::Request<Vec<u8>>,
        responder: wry::RequestAsyncResponder,
    ) {
        let body = self.try_handle_event(request).unwrap_or_default();
        responder.respond(wry::http::Response::new(body))
    }

    pub fn try_handle_event(
        &self,
        request: wry::http::Request<Vec<u8>>,
    ) -> Result<Vec<u8>, serde_json::Error> {
        let response = match serde_json::from_slice(request.body()) {
            Ok(event) => self.handle_html_event(event),
            Err(err) => {
                tracing::error!("Error parsing user_event: {:?}", err);
                SynchronousEventResponse::new(false)
            }
        };

        let body = match serde_json::to_vec(&response) {
            Ok(body) => body,
            Err(err) => {
                tracing::error!("failed to serialize SynchronousEventResponse: {err:?}");
                return Err(err);
            }
        };

        Ok(body)
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

        let recent_file = desktop_context.file_hover.clone();

        // check for a mounted event placeholder and replace it with a desktop specific element
        let as_any = match data {
            dioxus_html::EventData::Mounted => {
                let element = DesktopElement::new(element, desktop_context.clone());
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
        //
        // todo: move this to our launch function that's different for desktop and mobile
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
                let _: () = msg_send![window, setCollectionBehavior: NSWindowCollectionBehavior::NSWindowCollectionBehaviorManaged];
            }
        }

        let mut web_context = WebContext::new(cfg.data_dir.clone());
        let edit_queue = WryQueue::default();
        let asset_handlers = AssetHandlers::new();
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
            move |evt| {
                // Update the most recent file drop event - when the event comes in from the webview we can use the
                // most recent event to build a new event with the files in it.
                file_hover.set(evt);
                false
            }
        };

        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "ios",
            target_os = "android"
        ))]
        let mut webview = WebViewBuilder::new(&window);

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

        // The context will function as both the document and the context provider
        // But we need to disambiguate the types for rust's TypeId to downcast Rc<dyn Document> properly
        let desktop_context = Rc::from(DesktopService::new(
            webview,
            window,
            shared.clone(),
            asset_handlers,
            file_hover,
        ));
        let as_document: Rc<dyn Document> = desktop_context.clone() as Rc<dyn Document>;

        // Provide the desktop context to the virtual dom and edit handler
        edits.set_desktop_context(desktop_context.clone());

        dom.in_runtime(|| {
            ScopeId::ROOT.provide_context(desktop_context.clone());
            ScopeId::ROOT.provide_context(as_document);
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
                let fut = self.dom.wait_for_work();
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => return,
                }
            }

            self.dom
                .render_immediate(&mut *self.edits.wry_queue.mutation_state_mut());
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
