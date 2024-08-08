use crate::menubar::DioxusMenu;
use crate::{
    app::SharedContext, assets::AssetHandlerRegistry, document::DesktopDocument, edits::EditQueue,
    file_upload::NativeFileHover, ipc::UserWindowEvent, protocol, waker::tao_waker, Config,
    DesktopContext, DesktopService,
};
use dioxus_core::{ScopeId, VirtualDom};
use dioxus_html::document::Document;
use futures_util::{pin_mut, FutureExt};
use std::{rc::Rc, task::Waker};
use wry::{RequestAsyncResponder, WebContext, WebViewBuilder};

pub(crate) struct WebviewInstance {
    pub dom: VirtualDom,
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

        // tao makes small windows for some reason, make them bigger
        if cfg.window.window.inner_size.is_none() {
            window = window.with_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0));
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
        let edit_queue = EditQueue::default();
        let file_hover = NativeFileHover::default();
        let asset_handlers = AssetHandlerRegistry::new(dom.runtime());
        let headless = !cfg.window.window.visible;

        // Rust :(
        let window_id = window.id();
        let custom_head = cfg.custom_head.clone();
        let index_file = cfg.custom_index.clone();
        let root_name = cfg.root_name.clone();
        let asset_handlers_ = asset_handlers.clone();
        let edit_queue_ = edit_queue.clone();
        let proxy_ = shared.proxy.clone();
        let file_hover_ = file_hover.clone();

        let request_handler = move |request, responder: RequestAsyncResponder| {
            // Try to serve the index file first
            let index_bytes = protocol::index_request(
                &request,
                custom_head.clone(),
                index_file.clone(),
                &root_name,
                headless,
            );

            // Otherwise, try to serve an asset, either from the user or the filesystem
            match index_bytes {
                Some(body) => responder.respond(body),
                None => protocol::desktop_handler(
                    request,
                    asset_handlers_.clone(),
                    &edit_queue_,
                    responder,
                ),
            }
        };

        let ipc_handler = move |payload: wry::http::Request<String>| {
            // defer the event to the main thread
            let body = payload.into_body();
            if let Ok(msg) = serde_json::from_str(&body) {
                _ = proxy_.send_event(UserWindowEvent::Ipc { id: window_id, msg });
            }
        };

        let file_drop_handler = move |evt| {
            // Update the most recent file drop event - when the event comes in from the webview we can use the
            // most recent event to build a new event with the files in it.
            file_hover_.set(evt);
            false
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
            if let Some(menu) = &cfg.menu {
                crate::menubar::init_menu_bar(menu, &window);
            }
            cfg.menu
        } else {
            None
        };

        let desktop_context = Rc::from(DesktopService::new(
            webview,
            window,
            shared.clone(),
            edit_queue,
            asset_handlers,
            file_hover,
        ));

        let provider: Rc<dyn Document> = Rc::new(DesktopDocument::new(desktop_context.clone()));

        dom.in_runtime(|| {
            ScopeId::ROOT.provide_context(desktop_context.clone());
            ScopeId::ROOT.provide_context(provider);
        });

        WebviewInstance {
            waker: tao_waker(shared.proxy.clone(), desktop_context.window.id()),
            desktop_context,
            dom,
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
            let edits_flushed_poll = self.desktop_context.edit_queue.poll_edits_flushed(&mut cx);
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
                .render_immediate(&mut *self.desktop_context.mutation_state.borrow_mut());
            self.desktop_context.send_edits();
        }
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn kick_stylsheets(&self) {
        // run eval in the webview to kick the stylesheets by appending a query string
        // we should do something less clunky than this
        _ = self
            .desktop_context
            .webview
            .evaluate_script("window.interpreter.kickAllStylesheetsOnPage()");
    }
}
