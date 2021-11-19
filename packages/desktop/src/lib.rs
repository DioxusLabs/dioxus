//! Dioxus Desktop Renderer
//!
//! Render the Dioxus VirtualDom using the platform's native WebView implementation.
//!

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use cfg::DesktopConfig;
use dioxus_core::*;
use serde::{Deserialize, Serialize};

pub use wry;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{ElementState, Event, StartCause, WindowEvent};
use wry::application::event_loop::{self, ControlFlow, EventLoop, EventLoopWindowTarget};
use wry::application::keyboard::{Key, KeyCode, ModifiersState};
use wry::application::menu::{MenuBar, MenuItem, MenuItemAttributes};
use wry::application::window::{Fullscreen, WindowId};
use wry::webview::{WebView, WebViewBuilder};
use wry::{
    application::menu,
    application::window::{Window, WindowBuilder},
    webview::{RpcRequest, RpcResponse},
};

mod cfg;
mod desktop_context;
mod dom;
mod escape;
mod events;

static HTML_CONTENT: &'static str = include_str!("./index.html");

pub fn launch(
    root: FC<()>,
    config_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) {
    launch_with_props(root, (), config_builder)
}

pub fn launch_with_props<P: Properties + 'static + Send + Sync>(
    root: FC<P>,
    props: P,
    builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) {
    run(root, props, builder)
}

#[derive(Serialize)]
struct Response<'a> {
    pre_rendered: Option<String>,
    edits: Vec<DomEdit<'a>>,
}

pub fn run<T: 'static + Send + Sync>(
    root: FC<T>,
    props: T,
    user_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) {
    // Generate the config
    let mut cfg = DesktopConfig::new();
    user_builder(&mut cfg);
    let DesktopConfig {
        window: window_cfg,
        manual_edits,
        pre_rendered,
        ..
    } = cfg;

    // All of our webview windows are stored in a way that we can look them up later
    // The "DesktopContext" will provide functionality for spawning these windows
    let mut webviews = HashMap::<WindowId, WebView>::new();
    let event_loop = EventLoop::new();

    let props_shared = Cell::new(Some(props));

    // create local modifier state
    let modifiers = ModifiersState::default();

    let quit_hotkey = Accelerator::new(SysMods::Cmd, KeyCode::KeyQ);

    let edit_queue = Arc::new(RwLock::new(VecDeque::new()));
    let is_ready: Arc<AtomicBool> = Default::default();

    let mut frame = 0;

    event_loop.run(move |window_event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match window_event {
            Event::NewEvents(StartCause::Init) => {
                let window = create_window(event_loop, &window_cfg);
                let window_id = window.id();
                let sender =
                    launch_vdom_with_tokio(root, props_shared.take().unwrap(), edit_queue.clone());
                let webview = create_webview(window, is_ready.clone(), sender);
                webviews.insert(window_id, webview);
            }

            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Destroyed { .. } => {
                    webviews.remove(&window_id);
                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::Moved(pos) => {
                    //
                }

                WindowEvent::KeyboardInput { event, .. } => {
                    if quit_hotkey.matches(&modifiers, &event.physical_key) {
                        webviews.remove(&window_id);
                        if webviews.is_empty() {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }

                WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                    if let Some(view) = webviews.get_mut(&window_id) {
                        let _ = view.resize();
                    }
                }
                // TODO: we want to shuttle all of these events into the user's app
                _ => {}
            },

            Event::MainEventsCleared => {
                // I hate this ready hack but it's needed to wait for the "onload" to occur
                // We can't run any initializion scripts because the window isn't ready yet?
                if is_ready.load(std::sync::atomic::Ordering::Relaxed) {
                    let mut queue = edit_queue.write().unwrap();
                    let (id, view) = webviews.iter_mut().next().unwrap();
                    while let Some(edit) = queue.pop_back() {
                        view.evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                            .unwrap();
                    }
                } else {
                    println!("waiting for onload {:?}", frame);
                    frame += 1;
                }
            }
            Event::Resumed => {}
            Event::Suspended => {}
            Event::LoopDestroyed => {}

            _ => {}
        }
    })
}

// Create a new tokio runtime on a dedicated thread and then launch the apps VirtualDom.
pub(crate) fn launch_vdom_with_tokio<P: Send + 'static>(
    root: FC<P>,
    props: P,
    edit_queue: Arc<RwLock<VecDeque<String>>>,
) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
    let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();
    let return_sender = sender.clone();

    std::thread::spawn(move || {
        // We create the runtim as multithreaded, so you can still "spawn" onto multiple threads
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
            let mut dom = VirtualDom::new_with_props_and_scheduler(root, props, sender, receiver);

            let edits = dom.rebuild();

            edit_queue
                .write()
                .unwrap()
                .push_front(serde_json::to_string(&edits.edits).unwrap());

            loop {
                dom.wait_for_work().await;
                let mut muts = dom.work_with_deadline(|| false);
                while let Some(edit) = muts.pop() {
                    edit_queue
                        .write()
                        .unwrap()
                        .push_front(serde_json::to_string(&edit.edits).unwrap());
                }
            }
        })
    });

    return_sender
}

fn build_menu() -> MenuBar {
    // create main menubar menu
    let mut menu_bar_menu = MenuBar::new();

    // create `first_menu`
    let mut first_menu = MenuBar::new();

    first_menu.add_native_item(MenuItem::About("Todos".to_string()));
    first_menu.add_native_item(MenuItem::Services);
    first_menu.add_native_item(MenuItem::Separator);
    first_menu.add_native_item(MenuItem::Hide);
    first_menu.add_native_item(MenuItem::HideOthers);
    first_menu.add_native_item(MenuItem::ShowAll);

    first_menu.add_native_item(MenuItem::Quit);
    first_menu.add_native_item(MenuItem::CloseWindow);

    // create second menu
    let mut second_menu = MenuBar::new();

    // second_menu.add_submenu("Sub menu", true, my_sub_menu);
    second_menu.add_native_item(MenuItem::Copy);
    second_menu.add_native_item(MenuItem::Paste);
    second_menu.add_native_item(MenuItem::SelectAll);

    menu_bar_menu.add_submenu("First menu", true, first_menu);
    menu_bar_menu.add_submenu("Second menu", true, second_menu);

    menu_bar_menu
}

fn create_window(event_loop: &EventLoopWindowTarget<()>, cfg: &WindowBuilder) -> Window {
    let builder = cfg.clone().with_menu(build_menu());
    builder.build(event_loop).unwrap()
}

fn create_webview(
    window: Window,
    is_ready: Arc<AtomicBool>,
    sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
) -> WebView {
    WebViewBuilder::new(window)
        .unwrap()
        .with_url("wry://index.html")
        .unwrap()
        .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
            match req.method.as_str() {
                "user_event" => {
                    let event = events::trigger_from_serialized(req.params.unwrap());
                    log::debug!("User event: {:?}", event);
                    sender.unbounded_send(SchedulerMsg::UiEvent(event)).unwrap();
                }
                "initialize" => {
                    is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }
            // always driven through eval
            None
        })
        // .with_initialization_script(include_str!("./index.js"))
        // Any content that that uses the `wry://` scheme will be shuttled through this handler as a "special case"
        // For now, we only serve two pieces of content which get included as bytes into the final binary.
        .with_custom_protocol("wry".into(), move |request| {
            let path = request.uri().replace("wry://", "");
            let (data, meta) = match path.as_str() {
                "index.html" => (include_bytes!("./index.html").to_vec(), "text/html"),
                "index.html/index.js" => (include_bytes!("./index.js").to_vec(), "text/javascript"),
                _ => unimplemented!("path {}", path),
            };

            wry::http::ResponseBuilder::new().mimetype(meta).body(data)
        })
        .build()
        .unwrap()
}
