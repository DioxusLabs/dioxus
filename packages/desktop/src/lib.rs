//! Dioxus Desktop Renderer
//!
//! Render the Dioxus VirtualDom using the platform's native WebView implementation.
//!

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use cfg::DesktopConfig;
use dioxus_core::scheduler::SchedulerMsg;
use dioxus_core::*;
use serde::{Deserialize, Serialize};

pub use wry;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{Event, StartCause, WindowEvent};
use wry::application::event_loop::{self, ControlFlow, EventLoop};
use wry::application::keyboard::KeyCode;
use wry::application::menu::{MenuBar, MenuItem, MenuItemAttributes};
use wry::application::window::Fullscreen;
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
enum RpcEvent<'a> {
    Initialize { edits: Vec<DomEdit<'a>> },
}

#[derive(Debug)]
enum BridgeEvent {
    Initialize(serde_json::Value),
    Update(serde_json::Value),
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
        window,
        manual_edits,
        pre_rendered,
        ..
    } = cfg;

    // All of our webview windows are stored in a way that we can look them up later
    // The "DesktopContext" will provide functionality for spawning these windows
    let mut webviews = HashMap::new();
    let event_loop = EventLoop::new();

    let props_shared = Cell::new(Some(props));

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
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

                let quit_item = first_menu.add_item(
                    MenuItemAttributes::new("Quit")
                        .with_accelerators(&Accelerator::new(SysMods::Cmd, KeyCode::KeyQ)),
                );

                // create second menu
                let mut second_menu = MenuBar::new();

                // second_menu.add_submenu("Sub menu", true, my_sub_menu);
                second_menu.add_native_item(MenuItem::Copy);
                second_menu.add_native_item(MenuItem::Paste);
                second_menu.add_native_item(MenuItem::SelectAll);

                menu_bar_menu.add_submenu("First menu", true, first_menu);
                menu_bar_menu.add_submenu("Second menu", true, second_menu);

                let window = WindowBuilder::new()
                    .with_menu(menu_bar_menu)
                    .with_title("Dioxus App")
                    .build(event_loop)
                    .unwrap();
                let window_id = window.id();

                let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
                let my_props = props_shared.take().unwrap();

                let sender = launch_vdom_with_tokio(root, my_props, event_tx);

                let locked_receiver = Rc::new(RefCell::new(event_rx));

                let webview = WebViewBuilder::new(window)
                    .unwrap()
                    .with_url("wry://index.html")
                    .unwrap()
                    .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
                        let mut rx = (*locked_receiver).borrow_mut();
                        match req.method.as_str() {
                            "initiate" => {
                                if let Ok(BridgeEvent::Initialize(edits)) = rx.try_recv() {
                                    Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                                } else {
                                    None
                                }
                            }
                            "user_event" => {
                                let event = events::trigger_from_serialized(req.params.unwrap());
                                log::debug!("User event: {:?}", event);

                                sender.unbounded_send(SchedulerMsg::UiEvent(event)).unwrap();

                                if let Some(BridgeEvent::Update(edits)) = rx.blocking_recv() {
                                    log::info!("bridge received message {:?}", edits);
                                    Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                                } else {
                                    log::info!("none received message");
                                    None
                                }
                            }
                            _ => None,
                        }
                    })
                    // Any content that that uses the `wry://` scheme will be shuttled through this handler as a "special case"
                    // For now, we only serve two pieces of content which get included as bytes into the final binary.
                    .with_custom_protocol("wry".into(), move |request| {
                        let path = request.uri().replace("wry://", "");
                        let (data, meta) = match path.as_str() {
                            "index.html" => (include_bytes!("./index.html").to_vec(), "text/html"),
                            "index.html/index.js" => {
                                (include_bytes!("./index.js").to_vec(), "text/javascript")
                            }
                            _ => unimplemented!("path {}", path),
                        };

                        wry::http::ResponseBuilder::new().mimetype(meta).body(data)
                    })
                    .build()
                    .unwrap();

                webviews.insert(window_id, webview);
            }

            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                    if let Some(view) = webviews.get_mut(&window_id) {
                        let _ = view.resize();
                    }
                }
                // TODO: we want to shuttle all of these events into the user's app
                _ => {}
            },

            Event::MainEventsCleared => {}
            Event::Resumed => {}
            Event::Suspended => {}
            Event::LoopDestroyed => {}

            _ => {}
        }
    })
}

pub fn start<P: 'static + Send>(
    root: FC<P>,
    config_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> ((), ()) {
    //
    ((), ())
}

// Create a new tokio runtime on a dedicated thread and then launch the apps VirtualDom.
pub(crate) fn launch_vdom_with_tokio<P: Send + 'static>(
    root: FC<P>,
    props: P,
    event_tx: tokio::sync::mpsc::UnboundedSender<BridgeEvent>,
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
            let mut vir = VirtualDom::new_with_props_and_scheduler(root, props, sender, receiver);
            let _ = vir.get_event_sender();

            let edits = vir.rebuild();

            // the receiving end expects something along these lines
            #[derive(Serialize)]
            struct Evt<'a> {
                edits: Vec<DomEdit<'a>>,
            }

            let edit_string = serde_json::to_value(Evt { edits: edits.edits }).unwrap();

            event_tx
                .send(BridgeEvent::Initialize(edit_string))
                .expect("Sending should not fail");

            loop {
                vir.wait_for_work().await;
                // we're running on our own thread, so we don't need to worry about blocking anything
                // todo: maybe we want to schedule ourselves in
                // on average though, the virtualdom running natively is stupid fast

                let mut muts = vir.run_with_deadline(|| false);

                log::debug!("finished running with deadline");

                let mut edits = vec![];

                while let Some(edit) = muts.pop() {
                    log::debug!("sending message on channel with edit {:?}", edit);
                    let edit_string = serde_json::to_value(Evt { edits: edit.edits })
                        .expect("serializing edits should never fail");
                    edits.push(edit_string);
                }

                event_tx
                    .send(BridgeEvent::Update(serde_json::Value::Array(edits)))
                    .expect("Sending should not fail");
            }
        })
    });

    return_sender
}
