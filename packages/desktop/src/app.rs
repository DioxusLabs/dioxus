use crate::{
    cfg::{Config, WindowCloseBehaviour},
    desktop_context::{EventData, UserWindowEvent, WindowEventHandlers},
    edits::WebviewQueue,
    element::DesktopElement,
    file_upload::FileDialogRequest,
    ipc::IpcMessage,
    query::QueryResult,
    shortcut::{GlobalHotKeyEvent, ShortcutRegistry},
    webview::WebviewInstance,
};
use crossbeam_channel::Receiver;
use dioxus_core::{Component, ElementId, VirtualDom};
use dioxus_html::{native_bind::NativeFileEngine, FormData, HtmlEvent, MountedData};
use futures_util::{pin_mut, FutureExt};
use std::{cell::Cell, collections::HashMap, rc::Rc, sync::Arc};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::WindowId,
};

pub(crate) struct App<P> {
    // move the props into a cell so we can pop it out later to create the first window
    // iOS panics if we create a window before the event loop is started, so we toss them into a cell
    pub(crate) props: Cell<Option<P>>,
    pub(crate) cfg: Cell<Option<Config>>,

    // Stuff we need mutable access to
    pub(crate) control_flow: ControlFlow,
    pub(crate) is_visible_before_start: bool,
    pub(crate) root: Component<P>,
    pub(crate) webviews: HashMap<WindowId, WebviewInstance>,
    pub(crate) window_behavior: WindowCloseBehaviour,

    pub(crate) shared: SharedContext,
}

#[derive(Clone)]
pub struct SharedContext {
    pub(crate) event_handlers: WindowEventHandlers,
    pub(crate) pending_webviews: WebviewQueue,
    pub(crate) shortcut_manager: ShortcutRegistry,
    pub(crate) global_hotkey_channel: Receiver<GlobalHotKeyEvent>,
    pub(crate) proxy: EventLoopProxy<UserWindowEvent>,
    pub(crate) target: EventLoopWindowTarget<UserWindowEvent>,
}

impl<P: 'static> App<P> {
    pub fn new(cfg: Config, props: P, root: Component<P>) -> (EventLoop<UserWindowEvent>, Self) {
        let event_loop = EventLoopBuilder::<UserWindowEvent>::with_user_event().build();

        let mut app = Self {
            root,
            window_behavior: cfg.last_window_close_behaviour,
            is_visible_before_start: true,
            webviews: HashMap::new(),
            control_flow: ControlFlow::Wait,
            props: Cell::new(Some(props)),
            cfg: Cell::new(Some(cfg)),
            shared: SharedContext {
                event_handlers: WindowEventHandlers::default(),
                pending_webviews: WebviewQueue::default(),
                shortcut_manager: ShortcutRegistry::new(),
                global_hotkey_channel: GlobalHotKeyEvent::receiver().clone(),
                proxy: event_loop.create_proxy(),
                target: event_loop.clone(),
            },
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

        self.shared
            .event_handlers
            .apply_event(window_event, event_loop);

        _ = self
            .shared
            .global_hotkey_channel
            .try_recv()
            .map(|event| self.shared.shortcut_manager.call_handlers(event));
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn connect_hotreload(&mut self) {
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

    //
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
        let props = self.props.take().unwrap();
        let cfg = self.cfg.take().unwrap();

        self.is_visible_before_start = cfg.window.window.visible;

        let handler = WebviewInstance::new(
            cfg,
            VirtualDom::new_with_props(self.root, props),
            self.shared.clone(),
        );

        let id = handler.desktop_context.window.id();
        self.webviews.insert(id, handler);

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

        // check for a mounted event placeholder and replace it with a desktop specific element
        let as_any = match data {
            dioxus_html::EventData::Mounted => Rc::new(MountedData::new(DesktopElement::new(
                element,
                view.desktop_context.clone(),
                view.desktop_context.query.clone(),
            ))),
            _ => data.into_any(),
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

                for id in self.webviews.keys().copied().collect::<Vec<_>>() {
                    self.poll_vdom(id);
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

        let id = ElementId(file_dialog.target);
        let event_name = &file_dialog.event;
        let event_bubbles = file_dialog.bubbles;
        let files = file_dialog.get_file_event();

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

    /// Poll the virtualdom until it's pending
    ///
    /// The waker we give it is connected to the event loop, so it will wake up the event loop when it's ready to be polled again
    ///
    /// All IO is done on the tokio runtime we started earlier
    pub fn poll_vdom(&mut self, id: WindowId) {
        let Some(view) = self.webviews.get_mut(&id) else {
            return;
        };

        let mut cx = std::task::Context::from_waker(&view.waker);

        // Continously poll the virtualdom until it's pending
        // Wait for work will return Ready when it has edits to be sent to the webview
        // It will return Pending when it needs to be polled again - nothing is ready
        loop {
            {
                let fut = view.dom.wait_for_work();
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => return,
                }
            }

            view.desktop_context.send_edits(view.dom.render_immediate());
        }
    }
}

/// Different hide implementations per platform
#[allow(unused)]
pub fn hide_app_window(webview: &wry::WebView) {
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
