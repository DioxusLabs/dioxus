//! Dioxus Desktop Renderer
//!
//! Render the Dioxus VirtualDom using the platform's native WebView implementation.
//!

use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use cfg::DesktopConfig;
use dioxus_core::*;
use serde::{Deserialize, Serialize};
pub use wry;

use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop};
use wry::application::window::Fullscreen;
use wry::webview::WebViewBuilder;
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

enum RpcEvent<'a> {
    Initialize { edits: Vec<DomEdit<'a>> },
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
    run_with_edits(root, props, user_builder, None)
}

pub fn run_with_edits<
    F: for<'a, 'b> FnOnce(&'a mut DesktopConfig<'b>) -> &'a mut DesktopConfig<'b>,
    T: Properties + 'static + Send + Sync,
>(
    root: FC<T>,
    props: T,
    user_builder: F,
    redits: Option<Vec<DomEdit<'static>>>,
) -> anyhow::Result<()> {
    /*


    */

    let mut cfg = DesktopConfig::new();
    user_builder(&mut cfg);
    let DesktopConfig {
        window,
        manual_edits,
        pre_rendered,
    } = cfg;

    let event_loop = EventLoop::new();
    let window = window.build(&event_loop)?;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn the virtualdom onto its own thread
    // if it wants to spawn multithreaded tasks, it can use the executor directly
    std::thread::spawn(move || {
        //
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async move {
            let mut vir = VirtualDom::new_with_props(root, props);
            let channel = vir.get_event_sender();
            loop {
                vir.wait_for_work().await;
                let edits = vir.run_with_deadline(|| false);
                let edit_string = serde_json::to_string(&edits[0].edits).unwrap();
                event_tx.send(edit_string).unwrap();
            }
        })
    });

    let dioxus_requsted = Arc::new(AtomicBool::new(false));

    let webview = WebViewBuilder::new(window)?
        .with_url(&format!("data:text/html,{}", HTML_CONTENT))?
        .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
            match req.method.as_str() {
                "initiate" => {}
                "user_event" => {}
                _ => todo!("this message failed"),
            }
            todo!()
        })
        .build()?;

    event_loop.run(move |event, _, control_flow| {
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
    });

    Ok(())
}

// let edits = if let Some(edits) = &redits {
//     serde_json::to_value(edits).unwrap()
// } else {
//     let mut lock = vdom.write().unwrap();
//     // let mut reg_lock = registry.write().unwrap();
//     // Create the thin wrapper around the registry to collect the edits into
//     let mut real = dom::WebviewDom::new();
//     let pre = pre_rendered.clone();
//     let response = match pre {
//         Some(content) => {
//             lock.rebuild_in_place().unwrap();
//             Response {
//                 edits: Vec::new(),
//                 pre_rendered: Some(content),
//             }
//         }
//         None => {
//             //
//             let edits = {
//                 // let mut edits = Vec::new();
//                 todo!()
//                 // lock.rebuild(&mut real, &mut edits).unwrap();
//                 // edits
//             };
//             Response {
//                 edits,
//                 pre_rendered: None,
//             }
//         }
//     };
//     serde_json::to_value(&response).unwrap()
// };
// // Return the edits into the webview runtime
// Some(RpcResponse::new_result(req.id.take(), Some(edits)))

// log::debug!("User event received");
// // let registry = registry.clone();
// let vdom = vdom.clone();
// let response = async_std::task::block_on(async move {
//     let mut lock = vdom.write().unwrap();
//     // let mut reg_lock = registry.write().unwrap();
//     // a deserialized event
//     let data = req.params.unwrap();
//     log::debug!("Data: {:#?}", data);
//     let event = trigger_from_serialized(data);
//     // lock.queue_event(event);
//     // Create the thin wrapper around the registry to collect the edits into
//     let mut real = dom::WebviewDom::new();
//     // Serialize the edit stream
//     //
//     let mut edits = Vec::new();
//     // lock.run(&mut real, &mut edits)
//     //     .await
//     //     .expect("failed to progress");
//     let response = Response {
//         edits,
//         pre_rendered: None,
//     };
//     let response = serde_json::to_value(&response).unwrap();
//     // Give back the registry into its slot
//     // *reg_lock = Some(real.consume());
//     // Return the edits into the webview runtime
//     Some(RpcResponse::new_result(req.id.take(), Some(response)))
// });
// response
// // spawn a task to clean up the garbage
