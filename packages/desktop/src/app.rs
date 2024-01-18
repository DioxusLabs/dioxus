use crate::{
    config::{Config, WindowCloseBehaviour},
    element::DesktopElement,
    event_handlers::WindowEventHandlers,
    file_upload::FileDialogRequest,
    ipc::IpcMessage,
    ipc::{EventData, UserWindowEvent},
    query::QueryResult,
    shortcut::{GlobalHotKeyEvent, ShortcutRegistry},
    webview::WebviewInstance,
};
use crossbeam_channel::Receiver;
use dioxus_core::ElementId;
use dioxus_core::VirtualDom;
use dioxus_html::{
    native_bind::NativeFileEngine, FileEngine, HasFileData, HasFormData, HtmlEvent,
    PlatformEventData,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::WindowId,
};

/// The single top-level object that manages all the running windows, assets, shortcuts, etc
pub(crate) struct App {
    // move the props into a cell so we can pop it out later to create the first window
    // iOS panics if we create a window before the event loop is started, so we toss them into a cell
    pub(crate) unmounted_dom: Cell<Option<VirtualDom>>,
    pub(crate) cfg: Cell<Option<Config>>,

    // Stuff we need mutable access to
    pub(crate) control_flow: ControlFlow,
    pub(crate) is_visible_before_start: bool,
    pub(crate) window_behavior: WindowCloseBehaviour,
    pub(crate) webviews: HashMap<WindowId, WebviewInstance>,

    /// This single blob of state is shared between all the windows so they have access to the runtime state
    ///
    /// This includes stuff like the event handlers, shortcuts, etc as well as ways to modify *other* windows
    pub(crate) shared: Rc<SharedContext>,
}

/// A bundle of state shared between all the windows, providing a way for us to communicate with running webview.
///
/// Todo: everything in this struct is wrapped in Rc<>, but we really only need the one top-level refcell
pub(crate) struct SharedContext {
    pub(crate) event_handlers: WindowEventHandlers,
    pub(crate) pending_webviews: RefCell<Vec<WebviewInstance>>,
    pub(crate) shortcut_manager: ShortcutRegistry,
    pub(crate) global_hotkey_channel: Receiver<GlobalHotKeyEvent>,
    pub(crate) proxy: EventLoopProxy<UserWindowEvent>,
    pub(crate) target: EventLoopWindowTarget<UserWindowEvent>,
}

impl App {
    pub fn new(cfg: Config, virtual_dom: VirtualDom) -> (EventLoop<UserWindowEvent>, Self) {
        let event_loop = EventLoopBuilder::<UserWindowEvent>::with_user_event().build();

        let app = Self {
            window_behavior: cfg.last_window_close_behaviour,
            is_visible_before_start: true,
            webviews: HashMap::new(),
            control_flow: ControlFlow::Wait,
            unmounted_dom: Cell::new(Some(virtual_dom)),
            cfg: Cell::new(Some(cfg)),
            shared: Rc::new(SharedContext {
                event_handlers: WindowEventHandlers::default(),
                pending_webviews: Default::default(),
                shortcut_manager: ShortcutRegistry::new(),
                global_hotkey_channel: GlobalHotKeyEvent::receiver().clone(),
                proxy: event_loop.create_proxy(),
                target: event_loop.clone(),
            }),
        };

        // Set the event converter
        dioxus_html::set_event_converter(Box::new(crate::events::SerializedHtmlEventConverter));

        // Allow hotreloading to work - but only in debug mode
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        app.connect_hotreload();

        (event_loop, app)
    }

    pub fn tick(&mut self, window_event: &Event<'_, UserWindowEvent>) {
        self.control_flow = ControlFlow::Wait;

        self.shared
            .event_handlers
            .apply_event(window_event, &self.shared.target);

        if let Ok(event) = self.shared.global_hotkey_channel.try_recv() {
            self.shared.shortcut_manager.call_handlers(event);
        }
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn connect_hotreload(&self) {
        dioxus_hot_reload::connect({
            let proxy = self.shared.proxy.clone();
            move |template| {
                let _ = proxy.send_event(UserWindowEvent(
                    EventData::HotReloadEvent(template),
                    unsafe { WindowId::dummy() },
                ));
            }
        });
    }

    pub fn handle_new_window(&mut self) {
        for handler in self.shared.pending_webviews.borrow_mut().drain(..) {
            let id = handler.desktop_context.window.id();
            self.webviews.insert(id, handler);
            _ = self
                .shared
                .proxy
                .send_event(UserWindowEvent(EventData::Poll, id));
        }
    }

    pub fn handle_close_requested(&mut self, id: WindowId) {
        use WindowCloseBehaviour::*;

        match self.window_behavior {
            LastWindowExitsApp => {
                self.webviews.remove(&id);
                if self.webviews.is_empty() {
                    self.control_flow = ControlFlow::Exit
                }
            }

            LastWindowHides => {
                let Some(webview) = self.webviews.get(&id) else {
                    return;
                };
                hide_app_window(&webview.desktop_context.webview);
            }

            CloseWindow => {
                self.webviews.remove(&id);
            }
        }
    }

    pub fn window_destroyed(&mut self, id: WindowId) {
        self.webviews.remove(&id);

        if matches!(
            self.window_behavior,
            WindowCloseBehaviour::LastWindowExitsApp
        ) && self.webviews.is_empty()
        {
            self.control_flow = ControlFlow::Exit
        }
    }

    pub fn handle_start_cause_init(&mut self) {
        let virtual_dom = self.unmounted_dom.take().unwrap();
        let cfg = self.cfg.take().unwrap();

        self.is_visible_before_start = cfg.window.window.visible;

        let webview = WebviewInstance::new(cfg, virtual_dom, self.shared.clone());

        let id = webview.desktop_context.window.id();
        self.webviews.insert(id, webview);

        _ = self
            .shared
            .proxy
            .send_event(UserWindowEvent(EventData::Poll, id));
    }

    pub fn handle_browser_open(&mut self, msg: IpcMessage) {
        if let Some(temp) = msg.params().as_object() {
            if temp.contains_key("href") {
                let open = webbrowser::open(temp["href"].as_str().unwrap());
                if let Err(e) = open {
                    tracing::error!("Open Browser error: {:?}", e);
                }
            }
        }
    }

    pub fn handle_initialize_msg(&mut self, id: WindowId) {
        let view = self.webviews.get_mut(&id).unwrap();
        view.dom
            .rebuild(&mut *view.desktop_context.mutation_state.borrow_mut());
        view.desktop_context.send_edits();
        view.desktop_context
            .window
            .set_visible(self.is_visible_before_start);
    }

    pub fn handle_close_msg(&mut self, id: WindowId) {
        self.webviews.remove(&id);

        if self.webviews.is_empty() {
            self.control_flow = ControlFlow::Exit
        }
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

    pub fn handle_user_event_msg(&mut self, msg: IpcMessage, id: WindowId) {
        let parsed_params = serde_json::from_value(msg.params())
            .map_err(|err| tracing::error!("Error parsing user_event: {:?}", err));

        let Ok(evt) = parsed_params else { return };

        let HtmlEvent {
            element,
            name,
            bubbles,
            data,
        } = evt;

        let view = self.webviews.get_mut(&id).unwrap();
        let query = view.desktop_context.query.clone();

        // check for a mounted event placeholder and replace it with a desktop specific element
        let as_any = match data {
            dioxus_html::EventData::Mounted => {
                let element = DesktopElement::new(element, view.desktop_context.clone(), query);
                Rc::new(PlatformEventData::new(Box::new(element)))
            }
            _ => data.into_any(),
        };

        view.dom.handle_event(&name, as_any, element, bubbles);
        view.dom
            .render_immediate(&mut *view.desktop_context.mutation_state.borrow_mut());
        view.desktop_context.send_edits();
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn handle_hot_reload_msg(&mut self, msg: dioxus_hot_reload::HotReloadMsg) {
        match msg {
            dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                for webview in self.webviews.values_mut() {
                    webview.dom.replace_template(template);
                    webview.poll_vdom();
                }
            }
            dioxus_hot_reload::HotReloadMsg::Shutdown => {
                self.control_flow = ControlFlow::Exit;
            }
        }
    }

    pub fn handle_file_dialog_msg(&mut self, msg: IpcMessage, window: WindowId) {
        let Ok(file_dialog) = serde_json::from_value::<FileDialogRequest>(msg.params()) else {
            return;
        };
        struct DesktopFileUploadForm {
            files: Arc<NativeFileEngine>,
        }

        impl HasFileData for DesktopFileUploadForm {
            fn files(&self) -> Option<Arc<dyn FileEngine>> {
                Some(self.files.clone())
            }
        }

        impl HasFormData for DesktopFileUploadForm {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        let id = ElementId(file_dialog.target);
        let event_name = &file_dialog.event;
        let event_bubbles = file_dialog.bubbles;
        let files = file_dialog.get_file_event();

        let data = Rc::new(PlatformEventData::new(Box::new(DesktopFileUploadForm {
            files: Arc::new(NativeFileEngine::new(files)),
        })));

        let view = self.webviews.get_mut(&window).unwrap();

        if event_name == "change&input" {
            view.dom
                .handle_event("input", data.clone(), id, event_bubbles);
            view.dom.handle_event("change", data, id, event_bubbles);
        } else {
            view.dom.handle_event(event_name, data, id, event_bubbles);
        }

        view.dom
            .render_immediate(&mut *view.desktop_context.mutation_state.borrow_mut());
        view.desktop_context.send_edits();
    }

    /// Poll the virtualdom until it's pending
    ///
    /// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
    ///
    /// All IO is done on the tokio runtime we started earlier
    pub fn poll_vdom(&mut self, id: WindowId) {
        let Some(view) = self.webviews.get_mut(&id) else {
            return;
        };

        view.poll_vdom();
    }
}

/// Different hide implementations per platform
#[allow(unused)]
pub fn hide_app_window(window: &wry::WebView) {
    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        window.set_visible(false);
        // window.set_skip_taskbar(true);
    }

    #[cfg(target_os = "linux")]
    {
        use tao::platform::unix::WindowExtUnix;
        window.set_visible(false);
    }

    #[cfg(target_os = "macos")]
    {
        // window.set_visible(false); has the wrong behaviour on macOS
        // It will hide the window but not show it again when the user switches
        // back to the app. `NSApplication::hide:` has the correct behaviour
        use objc::runtime::Object;
        use objc::{msg_send, sel, sel_impl};
        objc::rc::autoreleasepool(|| unsafe {
            let app: *mut Object = msg_send![objc::class!(NSApplication), sharedApplication];
            let nil = std::ptr::null_mut::<Object>();
            let _: () = msg_send![app, hide: nil];
        });
    }
}
