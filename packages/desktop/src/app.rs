pub use crate::assets::{AssetFuture, AssetHandler, AssetRequest, AssetResponse};
pub use crate::cfg::{Config, WindowCloseBehaviour};
pub use crate::desktop_context::DesktopContext;
pub use crate::desktop_context::{window, DesktopService, WryEventHandler, WryEventHandlerId};
use crate::edits::{EditQueue, WebviewQueue};
use crate::element::DesktopElement;
use crate::eval::init_eval;
use crate::events::{IpcMessage, IpcMethod};
use crate::file_upload;
use crate::hooks::*;
use crate::query::QueryResult;
use crate::shortcut::GlobalHotKeyEvent;
use crate::shortcut::ShortcutRegistry;
pub use crate::shortcut::{ShortcutHandle, ShortcutId, ShortcutRegistryError};
use crate::{
    desktop_context::{EventData, UserWindowEvent, WindowEventHandlers},
    webview::WebviewHandler,
};
use dioxus_core::*;
use dioxus_html::{event_bubbles, MountedData};
use dioxus_html::{native_bind::NativeFileEngine, FormData, HtmlEvent};
use dioxus_interpreter_js::binary_protocol::Channel;
use futures_util::{pin_mut, FutureExt};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};
use rustc_hash::FxHashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicU16;
use std::task::Waker;
use std::{borrow::Borrow, cell::Cell};
use std::{collections::HashMap, sync::Arc};
pub use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};
pub use tao::window::WindowBuilder;
use tao::window::WindowId;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::ControlFlow,
};
use tao::{event_loop::EventLoopBuilder, window::Window};
use tokio::runtime::Builder;
pub use wry;
use wry::WebContext;
use wry::WebView;

pub struct App<P> {
    // move the props into a cell so we can pop it out later to create the first window
    // iOS panics if we create a window before the event loop is started
    pub(crate) props: Rc<Cell<Option<P>>>,
    pub(crate) cfg: Rc<Cell<Option<Config>>>,
    pub(crate) root: Component<P>,
    pub(crate) webviews: HashMap<WindowId, WebviewHandler>,
    pub(crate) event_handlers: WindowEventHandlers,
    pub(crate) queue: WebviewQueue,
    pub(crate) shortcut_manager: ShortcutRegistry,
    pub(crate) global_hotkey_channel: crossbeam_channel::Receiver<GlobalHotKeyEvent>,
    pub(crate) proxy: EventLoopProxy<UserWindowEvent>,
    pub(crate) window_behavior: WindowCloseBehaviour,
    pub(crate) control_flow: ControlFlow,
    pub(crate) is_visible_before_start: bool,
}

impl<P: 'static> App<P> {
    pub fn new(cfg: Config, props: P, root: Component<P>) -> (EventLoop<UserWindowEvent>, Self) {
        let event_loop = EventLoopBuilder::<UserWindowEvent>::with_user_event().build();

        let mut app = Self {
            root,
            window_behavior: cfg.last_window_close_behaviour,
            is_visible_before_start: true,
            webviews: HashMap::new(),
            event_handlers: WindowEventHandlers::default(),
            queue: WebviewQueue::default(),
            shortcut_manager: ShortcutRegistry::new(),
            global_hotkey_channel: GlobalHotKeyEvent::receiver().clone(),
            proxy: event_loop.create_proxy(),
            props: Rc::new(Cell::new(Some(props))),
            cfg: Rc::new(Cell::new(Some(cfg))),
            control_flow: ControlFlow::Wait,
        };

        #[cfg(all(feature = "hot-reload", debug_assertions))]
        app.connect_hotreload();

        (event_loop, app)
    }

    pub fn tick(
        &mut self,
        window_event: &Event<'_, UserWindowEvent>,
        event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    ) {
        self.control_flow = ControlFlow::Wait;

        self.event_handlers.apply_event(window_event, event_loop);

        _ = self
            .global_hotkey_channel
            .try_recv()
            .map(|event| self.shortcut_manager.call_handlers(event));
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn connect_hotreload(&mut self) {
        let proxy = self.proxy.clone();
        dioxus_hot_reload::connect({
            move |template| {
                let _ = proxy.send_event(UserWindowEvent(
                    EventData::HotReloadEvent(template),
                    unsafe { WindowId::dummy() },
                ));
            }
        });
    }

    //
    pub fn handle_new_window(&mut self) {
        for handler in self.queue.borrow_mut().drain(..) {
            let id = handler.desktop_context.window.id();
            self.webviews.insert(id, handler);
            _ = self.proxy.send_event(UserWindowEvent(EventData::Poll, id));
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

    pub fn handle_start_cause_init(&mut self, target: &EventLoopWindowTarget<UserWindowEvent>) {
        let props = self.props.take().unwrap();
        let cfg = self.cfg.take().unwrap();

        let dom = VirtualDom::new_with_props(self.root, props);

        self.is_visible_before_start = cfg.window.window.visible;

        let handler = crate::webview::create_new_window(
            cfg,
            dom,
            target,
            &self.proxy,
            &self.queue,
            &self.event_handlers,
            self.shortcut_manager.clone(),
        );

        let id = handler.desktop_context.window.id();
        self.webviews.insert(id, handler);

        _ = self.proxy.send_event(UserWindowEvent(EventData::Poll, id));
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
        view.desktop_context.send_edits(view.dom.rebuild());
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

        view.dom
            .base_scope()
            .consume_context::<DesktopContext>()
            .unwrap()
            .query
            .send(result);
    }

    pub fn handle_user_event_msg(&mut self, msg: IpcMessage, id: WindowId) {
        let params = msg.params();

        let evt = match serde_json::from_value::<HtmlEvent>(params) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!("Error parsing user_event: {:?}", err);
                return;
            }
        };

        let HtmlEvent {
            element,
            name,
            bubbles,
            data,
        } = evt;

        let view = self.webviews.get_mut(&id).unwrap();

        // check for a mounted event placeholder and replace it with a desktop specific element
        let as_any = if let dioxus_html::EventData::Mounted = &data {
            let query = view
                .dom
                .base_scope()
                .consume_context::<DesktopContext>()
                .unwrap()
                .query
                .clone();

            let element = DesktopElement::new(element, view.desktop_context.clone(), query);

            Rc::new(MountedData::new(element))
        } else {
            data.into_any()
        };

        view.dom.handle_event(&name, as_any, element, bubbles);
        view.desktop_context.send_edits(view.dom.render_immediate());
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn handle_hot_reload_msg(&mut self, msg: dioxus_hot_reload::HotReloadMsg) {
        match msg {
            dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                for webview in self.webviews.values_mut() {
                    webview.dom.replace_template(template);
                }

                let ids = self.webviews.keys().copied().collect::<Vec<_>>();

                for id in ids {
                    self.poll_vdom(id);
                }
            }
            dioxus_hot_reload::HotReloadMsg::Shutdown => {
                self.control_flow = ControlFlow::Exit;
            }
        }
    }

    pub fn handle_file_dialog_msg(&mut self, msg: IpcMessage, window: WindowId) {
        if let Ok(file_diolog) =
            serde_json::from_value::<file_upload::FileDialogRequest>(msg.params())
        {
            let id = ElementId(file_diolog.target);
            let event_name = &file_diolog.event;
            let event_bubbles = file_diolog.bubbles;
            let files = file_upload::get_file_event(&file_diolog);
            let data = Rc::new(FormData {
                value: Default::default(),
                values: Default::default(),
                files: Some(Arc::new(NativeFileEngine::new(files))),
            });

            let view = self.webviews.get_mut(&window).unwrap();

            if event_name == "change&input" {
                view.dom
                    .handle_event("input", data.clone(), id, event_bubbles);
                view.dom.handle_event("change", data, id, event_bubbles);
            } else {
                view.dom.handle_event(event_name, data, id, event_bubbles);
            }

            view.desktop_context.send_edits(view.dom.render_immediate());
        }
    }

    /// Poll the virtualdom until it's pending
    ///
    /// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
    ///
    /// All IO is done on the tokio runtime we started earlier
    pub fn poll_vdom(&mut self, id: WindowId) {
        let view = self.webviews.get_mut(&id).unwrap();

        let mut cx = std::task::Context::from_waker(&view.waker);

        loop {
            {
                let fut = view.dom.wait_for_work();
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => break,
                }
            }

            view.desktop_context.send_edits(view.dom.render_immediate());
        }
    }
}

/// Different hide implementations per platform
#[allow(unused)]
pub fn hide_app_window(webview: &WebView) {
    #[cfg(target_os = "windows")]
    {
        use wry::application::platform::windows::WindowExtWindows;
        window.set_visible(false);
        window.set_skip_taskbar(true);
    }

    #[cfg(target_os = "linux")]
    {
        use wry::application::platform::unix::WindowExtUnix;
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
