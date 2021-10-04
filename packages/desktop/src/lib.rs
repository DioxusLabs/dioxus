//! Dioxus Desktop Renderer
//!
//! Render the Dioxus VirtualDom using the platform's native WebView implementation.
//!

use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use cfg::DesktopConfig;
use dioxus_core::scheduler::SchedulerMsg;
use dioxus_core::*;
// use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};

mod logging;

pub use logging::set_up_logging;
pub use wry;

use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{self, ControlFlow, EventLoop};
use wry::application::window::Fullscreen;
use wry::webview::{WebView, WebViewBuilder};
use wry::{
    application::window::{Window, WindowBuilder},
    webview::{RpcRequest, RpcResponse},
};

mod cfg;
mod dom;
mod escape;
mod events;

static HTML_CONTENT: &'static str = include_str!("./index.html");

pub fn launch(
    root: FC<()>,
    config_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> anyhow::Result<()> {
    launch_with_props(root, (), config_builder)
}

pub fn launch_with_props<P: Properties + 'static + Send + Sync>(
    root: FC<P>,
    props: P,
    builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> anyhow::Result<()> {
    run(root, props, builder)
}

#[derive(Serialize)]
enum RpcEvent<'a> {
    Initialize { edits: Vec<DomEdit<'a>> },
}

enum BridgeEvent {
    Initialize(serde_json::Value),
    Update(serde_json::Value),
}

#[derive(Serialize)]
struct Response<'a> {
    pre_rendered: Option<String>,
    edits: Vec<DomEdit<'a>>,
}

pub fn run<T: Properties + 'static + Send + Sync>(
    root: FC<T>,
    props: T,
    user_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> anyhow::Result<()> {
    let mut cfg = DesktopConfig::new();
    user_builder(&mut cfg);
    let DesktopConfig {
        window,
        manual_edits,
        pre_rendered,
        ..
    } = cfg;

    let event_loop = EventLoop::new();
    let window = window.build(&event_loop)?;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

    let sender = launch_vdom_with_tokio(root, props, event_tx.clone());

    let locked_receiver = Rc::new(RefCell::new(event_rx));

    let webview = WebViewBuilder::new(window)?
        .with_url("wry://src/index.html")?
        .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
            match req.method.as_str() {
                "initiate" => {
                    //
                    let mut rx = (*locked_receiver).borrow_mut();

                    match rx.try_recv() {
                        Ok(BridgeEvent::Initialize(edits)) => {
                            Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                        }
                        _ => None,
                    }
                }
                "user_event" => {
                    //
                    let data = req.params.unwrap();
                    log::debug!("Data: {:#?}", data);
                    let event = events::trigger_from_serialized(data);
                    sender.unbounded_send(SchedulerMsg::UiEvent(event)).unwrap();

                    let mut rx = (*locked_receiver).borrow_mut();

                    match rx.blocking_recv() {
                        Some(BridgeEvent::Update(edits)) => {
                            log::info!("Passing response back");
                            Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                        }
                        None => {
                            log::error!("Sender half is gone");
                            None
                        }
                        _ => {
                            log::error!("No update event received");
                            None
                        }
                    }
                }
                _ => todo!("this message failed"),
            }
        })
        // this isn't quite portable unfortunately :(
        // todo: figure out a way to allow us to create the index.html with the index.js file separately
        // it's a bit easier to hack with
        .with_custom_protocol("wry".into(), move |request| {
            use std::fs::{canonicalize, read};
            use wry::http::ResponseBuilder;
            // Remove url scheme
            let path = request.uri().replace("wry://", "");
            // Read the file content from file path
            let content = read(canonicalize(&path)?)?;

            // Return asset contents and mime types based on file extentions
            // If you don't want to do this manually, there are some crates for you.
            // Such as `infer` and `mime_guess`.
            let (data, meta) = if path.ends_with(".html") {
                (content, "text/html")
            } else if path.ends_with(".js") {
                (content, "text/javascript")
            } else if path.ends_with(".png") {
                (content, "image/png")
            } else {
                unimplemented!();
            };

            ResponseBuilder::new().mimetype(meta).body(data)
        })
        .build()?;

    run_event_loop(event_loop, webview, event_tx);

    Ok(())
}

pub fn start<P: 'static + Send>(
    root: FC<P>,
    config_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> ((), ()) {
    //
    ((), ())
}

// Create a new tokio runtime on a dedicated thread and then launch the apps VirtualDom.
fn launch_vdom_with_tokio<C: Send + 'static>(
    root: FC<C>,
    props: C,
    event_tx: tokio::sync::mpsc::UnboundedSender<BridgeEvent>,
) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
    // Spawn the virtualdom onto its own thread
    // if it wants to spawn multithreaded tasks, it can use the executor directly

    let (sender, receiver) = futures_channel::mpsc::unbounded::<SchedulerMsg>();

    let sender_2 = sender.clone();
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

            #[derive(Serialize)]
            struct Evt<'a> {
                edits: Vec<DomEdit<'a>>,
            }

            // let msg = RpcEvent::Initialize { edits: edits.edits };
            let edit_string = serde_json::to_value(Evt { edits: edits.edits }).unwrap();
            match event_tx.send(BridgeEvent::Initialize(edit_string)) {
                Ok(_) => {}
                Err(_) => {}
            }

            loop {
                vir.wait_for_work().await;
                log::info!("{}", vir);

                let mut muts = vir.run_with_deadline(|| false);
                log::info!("muts {:#?}", muts);
                while let Some(edit) = muts.pop() {
                    let edit_string = serde_json::to_value(Evt { edits: edit.edits }).unwrap();
                    match event_tx.send(BridgeEvent::Update(edit_string)) {
                        Ok(_) => {}
                        Err(er) => {
                            log::error!("Sending should not fail {}", er);
                        }
                    }
                }

                log::info!("mutations sent on channel");
            }
        })
    });

    sender_2
}

fn run_event_loop(
    event_loop: EventLoop<()>,
    webview: WebView,
    event_tx: tokio::sync::mpsc::UnboundedSender<BridgeEvent>,
) {
    let _ = event_tx.clone();
    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                    let _ = webview.resize();
                }
                _ => {}
            },

            Event::MainEventsCleared => {
                webview.resize();
                // window.request_redraw();
            }

            _ => {}
        }
    })
}
