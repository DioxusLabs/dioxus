use crate::file_upload::{DesktopFileData, DesktopFileDragEvent};
use crate::menubar::DioxusMenu;
use crate::PendingDesktopContext;
use crate::{
    app::SharedContext, assets::AssetHandlerRegistry, edits::WryQueue,
    file_upload::NativeFileHover, ipc::UserWindowEvent, protocol, waker::tao_waker, Config,
    DesktopContext, DesktopService,
};
use crate::{document::DesktopDocument, WeakDesktopContext};
use crate::{element::DesktopElement, file_upload::DesktopFormData};
use base64::prelude::BASE64_STANDARD;
use dioxus_core::{consume_context, provide_context, Runtime, ScopeId, VirtualDom};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use dioxus_hooks::to_owned;
use dioxus_html::{FileData, FormValue, HtmlEvent, PlatformEventData, SerializedFileData};
use futures_util::{pin_mut, FutureExt};
use std::sync::{atomic::AtomicBool, Arc};
use std::{cell::OnceCell, time::Duration};
use std::{rc::Rc, task::Waker};
use wry::{DragDropEvent, RequestAsyncResponder, WebContext, WebViewBuilder, WebViewId};

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
                    "Error parsing user_event: {:?}. \n Contents: {:?}, \nraw: {:#?}",
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
        let hovered_file = desktop_context.file_hover.clone();

        // check for a mounted event placeholder and replace it with a desktop specific element
        // For mounted events, we create MountedData directly so we can retrieve cleanup after handler returns
        if matches!(data, dioxus_html::EventData::Mounted) {
            let desktop_element =
                DesktopElement::new(element, desktop_context.clone(), query.clone());
            // Create MountedData directly so we can access cleanup after handler returns
            let mounted_data = Rc::new(dioxus_html::MountedData::new(desktop_element));

            let event = dioxus_core::Event::new(
                mounted_data.clone() as Rc<dyn std::any::Any>,
                bubbles,
            );
            self.runtime.handle_event(&name, event.clone(), element);

            // Retrieve cleanup from shared MountedData after handler returns
            if let Some(cleanup) = mounted_data.take_cleanup() {
                desktop_context
                    .element_cleanup_closures
                    .borrow_mut()
                    .insert(element, cleanup);
            }

            return SynchronousEventResponse::new(!event.default_action_enabled());
        }

        let as_any = match data {
            dioxus_html::EventData::Mounted => {
                unreachable!("Handled above")
            }
            dioxus_html::EventData::Form(form) => {
                Rc::new(PlatformEventData::new(Box::new(DesktopFormData {
                    value: form.value,
                    valid: form.valid,
                    values: form
                        .values
                        .into_iter()
                        .map(|obj| {
                            if let Some(text) = obj.text {
                                return (obj.key, FormValue::Text(text));
                            }

                            if let Some(file_data) = obj.file {
                                if file_data.path.capacity() == 0 {
                                    return (obj.key, FormValue::File(None));
                                }

                                return (
                                    obj.key,
                                    FormValue::File(Some(FileData::new(DesktopFileData(
                                        file_data.path,
                                    )))),
                                );
                            };

                            (obj.key, FormValue::Text(String::new()))
                        })
                        .collect(),
                })))
            }
            // Which also includes drops...
            dioxus_html::EventData::Drag(ref drag) => {
                // we want to override this with a native file engine, provided by the most recent drag event
                let full_file_paths = hovered_file.current_paths();

                let xfer_data = drag.data_transfer.clone();
                let new_file_data = xfer_data
                    .files
                    .iter()
                    .map(|f| {
                        let new_path = full_file_paths
                            .iter()
                            .find(|p| p.ends_with(&f.path))
                            .unwrap_or(&f.path);
                        SerializedFileData {
                            path: new_path.clone(),
                            ..f.clone()
                        }
                    })
                    .collect::<Vec<_>>();
                let new_xfer_data = dioxus_html::SerializedDataTransfer {
                    files: new_file_data,
                    ..xfer_data
                };

                Rc::new(PlatformEventData::new(Box::new(DesktopFileDragEvent {
                    mouse: drag.mouse.clone(),
                    data_transfer: new_xfer_data,
                    files: full_file_paths,
                })))
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
        mut dom: VirtualDom,
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
        if let Some(on_build) = cfg.on_window.as_mut() {
            on_build(window.clone(), &mut dom);
        }

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

            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            move |_id: WebViewId, request, responder: RequestAsyncResponder| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();

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
            let (proxy, window_id) = (shared.proxy.to_owned(), window.id());
            move |evt: DragDropEvent| {
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

        let page_loaded = AtomicBool::new(false);

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
                handler(a, b)
            });
        }

        for (name, handler) in cfg.asynchronous_protocols.drain(..) {
            #[cfg(feature = "tokio_runtime")]
            let tokio_rt = tokio::runtime::Handle::current();

            webview = webview.with_asynchronous_custom_protocol(name, move |a, b, c| {
                #[cfg(feature = "tokio_runtime")]
                let _guard = tokio_rt.enter();
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
            file_hover,
            cfg.window_close_behavior,
        ));

        // Provide the desktop context to the virtual dom and edit handler
        edits.set_desktop_context(Rc::downgrade(&desktop_context));
        let provider: Rc<dyn Document> = Rc::new(DesktopDocument::new(desktop_context.clone()));
        let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
        dom.in_scope(ScopeId::ROOT, || {
            provide_context(desktop_context.clone());
            provide_context(provider);
            provide_context(history_provider);
        });

        // Request an initial redraw
        desktop_context.window.request_redraw();

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

            self.edits.wry_queue.with_mutation_state_mut(|f| {
                // Wrap MutationState to invoke cleanup on free_id
                let mut wrapper = DesktopMutations {
                    inner: f,
                    desktop_context: &self.desktop_context,
                };
                self.dom.render_immediate(&mut wrapper);
            });
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

/// A webview that is queued to be created. We can't spawn webviews outside of the main event loop because it may
/// block on windows so we queue them into the shared context and then create them when the main event loop is ready.
pub(crate) struct PendingWebview {
    dom: VirtualDom,
    cfg: Config,
    sender: futures_channel::oneshot::Sender<DesktopContext>,
}

impl PendingWebview {
    pub(crate) fn new(dom: VirtualDom, cfg: Config) -> (Self, PendingDesktopContext) {
        let (sender, receiver) = futures_channel::oneshot::channel();
        let webview = Self { dom, cfg, sender };
        let pending = PendingDesktopContext { receiver };
        (webview, pending)
    }

    pub(crate) fn create_window(self, shared: &Rc<SharedContext>) -> WebviewInstance {
        let window = WebviewInstance::new(self.cfg, self.dom, shared.clone());

        let cx = window
            .dom
            .in_scope(ScopeId::ROOT, consume_context::<Rc<DesktopService>>);
        _ = self.sender.send(cx);

        window
    }
}

/// A wrapper around `MutationState` that invokes cleanup closures on `free_id`.
///
/// Desktop needs to invoke cleanup closures stored in `DesktopContext` when elements are freed.
/// Since `MutationState` doesn't have access to `DesktopContext`, we wrap it here.
struct DesktopMutations<'a> {
    inner: &'a mut dioxus_interpreter_js::MutationState,
    desktop_context: &'a DesktopContext,
}

impl dioxus_core::WriteMutations for DesktopMutations<'_> {
    fn append_children(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.inner.append_children(id, m);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: dioxus_core::ElementId) {
        self.inner.assign_node_id(path, id);
    }

    fn create_placeholder(&mut self, id: dioxus_core::ElementId) {
        self.inner.create_placeholder(id);
    }

    fn create_text_node(&mut self, value: &str, id: dioxus_core::ElementId) {
        self.inner.create_text_node(value, id);
    }

    fn load_template(
        &mut self,
        template: dioxus_core::Template,
        index: usize,
        id: dioxus_core::ElementId,
    ) {
        self.inner.load_template(template, index, id);
    }

    fn replace_node_with(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.inner.replace_node_with(id, m);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.inner.replace_placeholder_with_nodes(path, m);
    }

    fn insert_nodes_after(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.inner.insert_nodes_after(id, m);
    }

    fn insert_nodes_before(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.inner.insert_nodes_before(id, m);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &dioxus_core::AttributeValue,
        id: dioxus_core::ElementId,
    ) {
        self.inner.set_attribute(name, ns, value, id);
    }

    fn set_node_text(&mut self, value: &str, id: dioxus_core::ElementId) {
        self.inner.set_node_text(value, id);
    }

    fn create_event_listener(&mut self, name: &'static str, id: dioxus_core::ElementId) {
        self.inner.create_event_listener(name, id);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: dioxus_core::ElementId) {
        self.inner.remove_event_listener(name, id);
    }

    fn remove_node(&mut self, id: dioxus_core::ElementId) {
        self.inner.remove_node(id);
    }

    fn push_root(&mut self, id: dioxus_core::ElementId) {
        self.inner.push_root(id);
    }

    fn free_id(&mut self, id: dioxus_core::ElementId) {
        // Invoke cleanup closure for this element
        self.desktop_context.invoke_cleanup(id);

        // Forward to inner MutationState to send to JS
        self.inner.free_id(id);
    }
}
