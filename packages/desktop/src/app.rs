pub use crate::cfg::{Config, WindowCloseBehaviour};
pub use crate::desktop_context::DesktopContext;
pub use crate::desktop_context::{
    use_window, use_wry_event_handler, window, DesktopService, WryEventHandler, WryEventHandlerId,
};
use crate::desktop_context::{EventData, UserWindowEvent, WebviewQueue, WindowEventHandlers};
use crate::events::{IpcMessage, KnownIpcMethod};
use crate::file_upload;
use crate::query::QueryResult;
use crate::shortcut::GlobalHotKeyEvent;
use dioxus_core::*;
use dioxus_html::{event_bubbles, MountedData};
use dioxus_html::{native_bind::NativeFileEngine, FormData, HtmlEvent};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};
// use dioxus_interpreter_js::binary_protocol::Channel;
use crate::element::DesktopElement;
use crate::eval::init_eval;
pub use crate::protocol::{
    use_asset_handler, AssetFuture, AssetHandler, AssetRequest, AssetResponse,
};
use crate::shortcut::ShortcutRegistry;
pub use crate::shortcut::{use_global_shortcut, ShortcutHandle, ShortcutId, ShortcutRegistryError};
use futures_util::{pin_mut, FutureExt};
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

impl<P> App<P> {
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
        // Set the control flow here, but make sure to update it at the end of the match
        self.control_flow = ControlFlow::Wait;

        self.event_handlers.apply_event(window_event, event_loop);

        if let Ok(event) = self.global_hotkey_channel.try_recv() {
            self.shortcut_manager.call_handlers(event);
        }
    }

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

        let handler = create_new_window(
            cfg,
            target,
            &self.proxy,
            dom,
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
        send_edits(view.dom.rebuild(), &view.desktop_context);
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

    pub fn handle_poll_msg(&mut self, id: WindowId) {
        self.poll_vdom(id);
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

        send_edits(view.dom.render_immediate(), &view.desktop_context);
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn handle_hot_reload_msg(&mut self, msg: dioxus_hot_reload::HotReloadMsg) {
        match msg {
            dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                for webview in self.webviews.values_mut() {
                    webview.dom.replace_template(template);

                    // poll_vdom(webview);
                    todo!()
                }
            }
            dioxus_hot_reload::HotReloadMsg::Shutdown => {
                self.control_flow = ControlFlow::Exit;
            }
        }
    }

    pub fn handle_file_dialog_msg(&self, msg: IpcMessage, window: WindowId) {
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

            todo!()
            // send_edits(view.dom.render_immediate(), &view.desktop_context);
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

            send_edits(view.dom.render_immediate(), &view.desktop_context);
        }
    }
}

pub fn create_new_window(
    mut cfg: Config,
    event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    proxy: &EventLoopProxy<UserWindowEvent>,
    dom: VirtualDom,
    queue: &WebviewQueue,
    event_handlers: &WindowEventHandlers,
    shortcut_manager: ShortcutRegistry,
) -> WebviewHandler {
    let (webview, web_context, asset_handlers, edit_queue, window) =
        crate::webview::build(&mut cfg, event_loop, proxy.clone());

    let desktop_context = Rc::from(DesktopService::new(
        window,
        webview,
        proxy.clone(),
        event_loop.clone(),
        queue.clone(),
        event_handlers.clone(),
        shortcut_manager,
        edit_queue,
        asset_handlers,
    ));

    let cx = dom.base_scope();
    cx.provide_context(desktop_context.clone());

    // Init eval
    init_eval(cx);

    WebviewHandler {
        // We want to poll the virtualdom and the event loop at the same time, so the waker will be connected to both
        waker: crate::waker::tao_waker(proxy, desktop_context.window.id()),
        desktop_context,
        dom,
        _web_context: web_context,
    }
}

pub struct WebviewHandler {
    dom: VirtualDom,
    desktop_context: DesktopContext,
    waker: Waker,

    // Wry assumes the webcontext is alive for the lifetime of the webview.
    // We need to keep the webcontext alive, otherwise the webview will crash
    _web_context: WebContext,
}

/// Send a list of mutations to the webview
pub fn send_edits(edits: Mutations, desktop_context: &DesktopContext) {
    let mut channel = desktop_context.channel.borrow_mut();
    let mut templates = desktop_context.templates.borrow_mut();
    if let Some(bytes) = apply_edits(
        edits,
        &mut channel,
        &mut templates,
        &desktop_context.max_template_count,
    ) {
        desktop_context.edit_queue.add_edits(bytes)
    }
}

pub struct Channel {}

pub fn apply_edits(
    mutations: Mutations,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &AtomicU16,
) -> Option<Vec<u8>> {
    use dioxus_core::Mutation::*;
    if mutations.templates.is_empty() && mutations.edits.is_empty() {
        return None;
    }
    for template in mutations.templates {
        add_template(&template, channel, templates, max_template_count);
    }
    for edit in mutations.edits {
        match edit {
            AppendChildren { id, m } => channel.append_children(id.0 as u32, m as u16),
            AssignId { path, id } => channel.assign_id(path, id.0 as u32),
            CreatePlaceholder { id } => channel.create_placeholder(id.0 as u32),
            CreateTextNode { value, id } => channel.create_text_node(value, id.0 as u32),
            HydrateText { path, value, id } => channel.hydrate_text(path, value, id.0 as u32),
            LoadTemplate { name, index, id } => {
                if let Some(tmpl_id) = templates.get(name) {
                    channel.load_template(*tmpl_id, index as u16, id.0 as u32)
                }
            }
            ReplaceWith { id, m } => channel.replace_with(id.0 as u32, m as u16),
            ReplacePlaceholder { path, m } => channel.replace_placeholder(path, m as u16),
            InsertAfter { id, m } => channel.insert_after(id.0 as u32, m as u16),
            InsertBefore { id, m } => channel.insert_before(id.0 as u32, m as u16),
            SetAttribute {
                name,
                value,
                id,
                ns,
            } => match value {
                BorrowedAttributeValue::Text(txt) => {
                    channel.set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Float(f) => {
                    channel.set_attribute(id.0 as u32, name, &f.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Int(n) => {
                    channel.set_attribute(id.0 as u32, name, &n.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Bool(b) => channel.set_attribute(
                    id.0 as u32,
                    name,
                    if b { "true" } else { "false" },
                    ns.unwrap_or_default(),
                ),
                BorrowedAttributeValue::None => {
                    channel.remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
                }
                _ => unreachable!(),
            },
            SetText { value, id } => channel.set_text(id.0 as u32, value),
            NewEventListener { name, id, .. } => {
                channel.new_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            RemoveEventListener { name, id } => {
                channel.remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            Remove { id } => channel.remove(id.0 as u32),
            PushRoot { id } => channel.push_root(id.0 as u32),
        }
    }

    let bytes: Vec<_> = channel.export_memory().collect();
    channel.reset();
    Some(bytes)
}

pub fn add_template(
    template: &Template<'static>,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &AtomicU16,
) {
    let current_max_template_count = max_template_count.load(std::sync::atomic::Ordering::Relaxed);
    for root in template.roots.iter() {
        create_template_node(channel, root);
        templates.insert(template.name.to_owned(), current_max_template_count);
    }
    channel.add_templates(current_max_template_count, template.roots.len() as u16);

    max_template_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub fn create_template_node(channel: &mut Channel, v: &'static TemplateNode<'static>) {
    use TemplateNode::*;
    match v {
        Element {
            tag,
            namespace,
            attrs,
            children,
            ..
        } => {
            // Push the current node onto the stack
            match namespace {
                Some(ns) => channel.create_element_ns(tag, ns),
                None => channel.create_element(tag),
            }
            // Set attributes on the current node
            for attr in *attrs {
                if let TemplateAttribute::Static {
                    name,
                    value,
                    namespace,
                } = attr
                {
                    channel.set_top_attribute(name, value, namespace.unwrap_or_default())
                }
            }
            // Add each child to the stack
            for child in *children {
                create_template_node(channel, child);
            }
            // Add all children to the parent
            channel.append_children_to_top(children.len() as u16);
        }
        Text { text } => channel.create_raw_text(text),
        DynamicText { .. } => channel.create_raw_text("p"),
        Dynamic { .. } => channel.add_placeholder(),
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
