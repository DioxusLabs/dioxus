use crate::{
    context::{DesktopContext, ProxyType, UserEvent},
    event::WebKeyboardEvent,
};
use bevy::{
    ecs::world::WorldCell,
    math::IVec2,
    utils::HashMap,
    window::{Window as BevyWindow, WindowDescriptor, WindowId, WindowMode},
};
use dioxus_core::{Component as DioxusComponent, SchedulerMsg, VirtualDom};
use dioxus_desktop::{
    cfg::DesktopConfig,
    desktop_context::UserWindowEvent,
    events::{parse_ipc_message, trigger_from_serialized},
    protocol,
    tao::{
        dpi::{LogicalPosition, LogicalSize},
        error::ExternalError,
        event_loop::EventLoop,
        monitor::{MonitorHandle, VideoMode},
        window::{Fullscreen, Window as TaoWindow, WindowBuilder, WindowId as TaoWindowId},
    },
    wry::webview::{WebView, WebViewBuilder},
};
use futures_channel::mpsc;
use futures_intrusive::channel::shared::{Receiver, Sender};
use raw_window_handle::HasRawWindowHandle;
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct DioxusWindows {
    windows: HashMap<TaoWindowId, Window>,
    window_id_to_tao: HashMap<WindowId, TaoWindowId>,
    tao_to_window_id: HashMap<TaoWindowId, WindowId>,
    _not_send_sync: PhantomData<*const ()>,

    quit_app_on_close: bool,
}

impl Debug for DioxusWindows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DioxusWindows")
            .field("windonw keys", &self.windows.keys())
            .field("window_id_to_tao", &self.window_id_to_tao)
            .field("tao_to_window_id", &self.tao_to_window_id)
            .field("quit_app_on_close", &self.quit_app_on_close)
            .finish()
    }
}

impl DioxusWindows {
    pub fn get(&self, id: WindowId) -> Option<&Window> {
        self.window_id_to_tao
            .get(&id)
            .and_then(|id| self.windows.get(id))
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.window_id_to_tao
            .get(&id)
            .and_then(|id| self.windows.get_mut(id))
    }

    pub fn get_tao_window(&self, id: WindowId) -> Option<&TaoWindow> {
        self.get(id).and_then(|window| Some(window.tao_window()))
    }

    pub fn get_window_id(&self, id: TaoWindowId) -> Option<WindowId> {
        self.tao_to_window_id.get(&id).cloned()
    }

    pub fn create<CoreCommand, UICommand, Props>(
        &mut self,
        world: &WorldCell,
        window_id: WindowId,
        window_descriptor: &WindowDescriptor,
    ) -> BevyWindow
    where
        CoreCommand: 'static + Send + Sync + Clone + Debug,
        UICommand: 'static + Send + Sync + Clone + Debug,
        Props: 'static + Send + Sync + Copy,
    {
        let event_loop = world
            .get_non_send_mut::<EventLoop<UserEvent<CoreCommand>>>()
            .unwrap();
        let proxy = event_loop.create_proxy();

        let tao_window = Self::create_tao_window::<CoreCommand>(&event_loop, &window_descriptor);
        let tao_window_id = tao_window.id();

        let bevy_window = Self::create_bevy_window(window_id, &tao_window, &window_descriptor);
        let (dom_tx, edit_queue) =
            Self::spawn_virtual_dom::<CoreCommand, UICommand, Props>(world, proxy.clone());
        let (webview, is_ready) =
            Self::create_webview(world, window_descriptor, tao_window, proxy, dom_tx.clone());

        self.windows.insert(
            tao_window_id,
            Window::new(webview, dom_tx, is_ready, edit_queue),
        );
        self.window_id_to_tao.insert(window_id, tao_window_id);
        self.tao_to_window_id.insert(tao_window_id, window_id);

        bevy_window
    }

    pub fn get_fitting_videomode(monitor: &MonitorHandle, width: u32, height: u32) -> VideoMode {
        let mut modes = monitor.video_modes().collect::<Vec<_>>();

        fn abs_diff(a: u32, b: u32) -> u32 {
            if a > b {
                return a - b;
            }
            b - a
        }

        modes.sort_by(|a, b| {
            use std::cmp::Ordering::*;
            match abs_diff(a.size().width, width).cmp(&abs_diff(b.size().width, width)) {
                Equal => {
                    match abs_diff(a.size().height, height).cmp(&abs_diff(b.size().height, height))
                    {
                        Equal => b.refresh_rate().cmp(&a.refresh_rate()),
                        default => default,
                    }
                }
                default => default,
            }
        });

        modes.first().unwrap().clone()
    }

    pub fn get_best_videomode(monitor: &MonitorHandle) -> VideoMode {
        let mut modes = monitor.video_modes().collect::<Vec<_>>();
        modes.sort_by(|a, b| {
            use std::cmp::Ordering::*;
            match b.size().width.cmp(&a.size().width) {
                Equal => match b.size().height.cmp(&a.size().height) {
                    Equal => b.refresh_rate().cmp(&a.refresh_rate()),
                    default => default,
                },
                default => default,
            }
        });

        modes.first().unwrap().clone()
    }

    fn create_tao_window<CoreCommand>(
        event_loop: &EventLoop<UserEvent<CoreCommand>>,
        window_descriptor: &WindowDescriptor,
    ) -> TaoWindow
    where
        CoreCommand: Debug,
    {
        let mut tao_window_builder = WindowBuilder::new().with_title(&window_descriptor.title);

        tao_window_builder = match window_descriptor.mode {
            WindowMode::BorderlessFullscreen => tao_window_builder
                .with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor()))),
            WindowMode::Fullscreen => {
                tao_window_builder.with_fullscreen(Some(Fullscreen::Exclusive(
                    Self::get_best_videomode(&event_loop.primary_monitor().unwrap()),
                )))
            }
            WindowMode::SizedFullscreen => tao_window_builder.with_fullscreen(Some(
                Fullscreen::Exclusive(Self::get_fitting_videomode(
                    &event_loop.primary_monitor().unwrap(),
                    window_descriptor.width as u32,
                    window_descriptor.height as u32,
                )),
            )),
            _ => {
                let WindowDescriptor {
                    width,
                    height,
                    position,
                    scale_factor_override,
                    ..
                } = window_descriptor;

                if let Some(position) = position {
                    if let Some(sf) = scale_factor_override {
                        tao_window_builder = tao_window_builder.with_position(
                            LogicalPosition::new(position[0] as f64, position[1] as f64)
                                .to_physical::<f64>(*sf),
                        );
                    } else {
                        tao_window_builder = tao_window_builder.with_position(
                            LogicalPosition::new(position[0] as f64, position[1] as f64),
                        );
                    }
                }
                if let Some(sf) = scale_factor_override {
                    tao_window_builder
                        .with_inner_size(LogicalSize::new(*width, *height).to_physical::<f64>(*sf))
                } else {
                    tao_window_builder.with_inner_size(LogicalSize::new(*width, *height))
                }
            }
            .with_resizable(window_descriptor.resizable)
            .with_decorations(window_descriptor.decorations)
            .with_transparent(window_descriptor.transparent),
        };

        let constraints = window_descriptor.resize_constraints.check_constraints();
        let min_inner_size = LogicalSize {
            width: constraints.min_width,
            height: constraints.min_height,
        };
        let max_inner_size = LogicalSize {
            width: constraints.max_width,
            height: constraints.max_height,
        };

        let tao_window_builder =
            if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
                tao_window_builder
                    .with_min_inner_size(min_inner_size)
                    .with_max_inner_size(max_inner_size)
            } else {
                tao_window_builder.with_min_inner_size(min_inner_size)
            };

        let tao_window = tao_window_builder.build(&event_loop).unwrap();

        if window_descriptor.cursor_locked {
            match tao_window.set_cursor_grab(true) {
                Ok(_) => {}
                Err(ExternalError::NotSupported(_)) => {}
                Err(err) => Err(err).unwrap(),
            }
        }

        tao_window.set_cursor_visible(window_descriptor.cursor_visible);

        tao_window
    }

    fn create_bevy_window(
        window_id: WindowId,
        tao_window: &TaoWindow,
        window_descriptor: &WindowDescriptor,
    ) -> BevyWindow {
        BevyWindow::new(
            window_id,
            window_descriptor,
            tao_window.inner_size().width,
            tao_window.inner_size().height,
            tao_window.scale_factor(),
            tao_window
                .outer_position()
                .ok()
                .map(|position| IVec2::new(position.x, position.y)),
            tao_window.raw_window_handle(),
        )
    }

    fn spawn_virtual_dom<CoreCommand, UICommand, Props>(
        world: &WorldCell,
        proxy: ProxyType<CoreCommand>,
    ) -> (mpsc::UnboundedSender<SchedulerMsg>, Arc<Mutex<Vec<String>>>)
    where
        CoreCommand: 'static + Send + Sync + Clone + Debug,
        UICommand: 'static + Send + Sync + Clone + Debug,
        Props: 'static + Send + Sync + Copy,
    {
        let root = world
            .get_resource::<DioxusComponent<Props>>()
            .unwrap()
            .clone();
        let props = world.get_resource::<Props>().unwrap().clone();
        let core_tx = world.get_resource::<Sender<CoreCommand>>().unwrap().clone();
        let ui_rx = world.get_resource::<Receiver<UICommand>>().unwrap().clone();

        let (dom_tx, dom_rx) = mpsc::unbounded::<SchedulerMsg>();
        let context =
            DesktopContext::<CoreCommand, UICommand>::new(proxy.clone(), (core_tx, ui_rx));
        let edit_queue = Arc::new(Mutex::new(Vec::new()));

        let dom_tx_clone = dom_tx.clone();
        let edit_queue_clone = edit_queue.clone();

        std::thread::spawn(move || {
            Runtime::new().unwrap().block_on(async move {
                let mut dom =
                    VirtualDom::new_with_props_and_scheduler(root, props, (dom_tx_clone, dom_rx));

                dom.base_scope().provide_context(context.clone());

                let edits = dom.rebuild();

                edit_queue_clone
                    .lock()
                    .unwrap()
                    .push(serde_json::to_string(&edits.edits).unwrap());

                // Make sure the window is ready for any new updates
                proxy
                    .send_event(UserEvent::WindowEvent(UserWindowEvent::Update))
                    .expect("Failed to send UserWindowEvent::Update");

                loop {
                    dom.wait_for_work().await;

                    let muts = dom.work_with_deadline(|| false);

                    for edit in muts {
                        edit_queue_clone
                            .lock()
                            .unwrap()
                            .push(serde_json::to_string(&edit.edits).unwrap());
                    }

                    let _ = proxy.send_event(UserEvent::WindowEvent(UserWindowEvent::Update));
                }
            });
        });

        (dom_tx, edit_queue)
    }

    fn create_webview<CoreCommand>(
        world: &WorldCell,
        window_descriptor: &WindowDescriptor,
        tao_window: TaoWindow,
        proxy: ProxyType<CoreCommand>,
        dom_tx: mpsc::UnboundedSender<SchedulerMsg>,
    ) -> (WebView, Arc<AtomicBool>)
    where
        CoreCommand: 'static + Send + Sync + Clone + Debug,
    {
        // TODO: warn user to use WindowDescriptor instead (e.g. title, icon, etc.)
        let mut config = world.get_non_send_mut::<DesktopConfig>().unwrap();
        let is_ready = Arc::new(AtomicBool::new(false));

        let file_drop_handler = config.file_drop_handler.take();
        let custom_head = config.custom_head.clone();
        let resource_dir = config.resource_dir.clone();
        let index_file = config.custom_index.clone();
        let is_ready_clone = is_ready.clone();

        let mut webview = WebViewBuilder::new(tao_window)
            .unwrap()
            .with_transparent(window_descriptor.transparent)
            .with_url("dioxus://index.html/")
            .unwrap()
            .with_ipc_handler(move |_window: &TaoWindow, payload: String| {
                parse_ipc_message(&payload)
                    .map(|message| match message.method() {
                        "user_event" => {
                            let event = trigger_from_serialized(message.params());
                            dom_tx.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                        }
                        "keyboard_event" => {
                            let event = WebKeyboardEvent::from_value(message.params());
                            proxy.send_event(UserEvent::KeyboardEvent(event)).unwrap();
                        }
                        "initialize" => {
                            is_ready_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                            let _ =
                                proxy.send_event(UserEvent::WindowEvent(UserWindowEvent::Update));
                        }
                        "browser_open" => {
                            let data = message.params();
                            log::trace!("Open browser: {:?}", data);
                            if let Some(temp) = data.as_object() {
                                if temp.contains_key("href") {
                                    let url = temp.get("href").unwrap().as_str().unwrap();
                                    if let Err(e) = webbrowser::open(url) {
                                        log::error!("Open Browser error: {:?}", e);
                                    }
                                }
                            }
                        }
                        _ => {}
                    })
                    .unwrap_or_else(|| {
                        log::warn!("invalid IPC message received");
                    })
            })
            .with_custom_protocol(String::from("dioxus"), move |r| {
                protocol::desktop_handler(
                    r,
                    resource_dir.clone(),
                    custom_head.clone(),
                    index_file.clone(),
                )
            })
            .with_file_drop_handler(move |window, evet| {
                file_drop_handler
                    .as_ref()
                    .map(|handler| handler(window, evet))
                    .unwrap_or_default()
            });

        for (name, handler) in config.protocols.drain(..) {
            webview = webview.with_custom_protocol(name, handler)
        }

        if config.disable_context_menu {
            // in release mode, we don't want to show the dev tool or reload menus
            webview = webview.with_initialization_script(
                r#"
                        if (document.addEventListener) {
                            document.addEventListener('contextmenu', function(e) {
                                alert("You've tried to open context menu");
                                e.preventDefault();
                            }, false);
                        } else {
                            document.attachEvent('oncontextmenu', function() {
                                alert("You've tried to open context menu");
                                window.event.returnValue = false;
                            });
                        }
                    "#,
            )
        } else {
            // in debug, we are okay with the reload menu showing and dev tool
            webview = webview.with_devtools(true);
        }

        (webview.build().unwrap(), is_ready)
    }
}

pub struct Window {
    pub webview: WebView,
    pub dom_tx: mpsc::UnboundedSender<SchedulerMsg>,
    is_ready: Arc<AtomicBool>,
    edit_queue: Arc<Mutex<Vec<String>>>,
}

impl Window {
    fn new(
        webview: WebView,
        dom_tx: mpsc::UnboundedSender<SchedulerMsg>,
        is_ready: Arc<AtomicBool>,
        edit_queue: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            webview,
            dom_tx,
            is_ready,
            edit_queue,
        }
    }

    pub fn tao_window(&self) -> &TaoWindow {
        &self.webview.window()
    }

    pub fn try_load_ready_webview(&mut self) {
        if self.is_ready.load(std::sync::atomic::Ordering::Relaxed) {
            let mut queue = self.edit_queue.lock().unwrap();

            for edit in queue.drain(..) {
                self.webview
                    .evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                    .unwrap();
            }
        }
    }
}
