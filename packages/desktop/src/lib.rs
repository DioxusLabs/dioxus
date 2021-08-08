use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
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
use events::*;

static HTML_CONTENT: &'static str = include_str!("./index.html");

pub fn launch(
    root: FC<()>,
    builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> anyhow::Result<()> {
    launch_with_props(root, (), builder)
}
pub fn launch_with_props<P: Properties + 'static>(
    root: FC<P>,
    props: P,
    builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
) -> anyhow::Result<()> {
    WebviewRenderer::run(root, props, builder)
}

/// The `WebviewRenderer` provides a way of rendering a Dioxus Virtual DOM through a bridge to a Webview instance.
/// Components used in WebviewRenderer instances can directly use system libraries, access the filesystem, and multithread with ease.
pub struct WebviewRenderer<T> {
    /// The root component used to render the Webview
    root: FC<T>,
}
enum RpcEvent<'a> {
    Initialize { edits: Vec<DomEdit<'a>> },
}

#[derive(Serialize)]
struct Response<'a> {
    pre_rendered: Option<String>,
    edits: Vec<DomEdit<'a>>,
}

impl<T: Properties + 'static> WebviewRenderer<T> {
    pub fn run(
        root: FC<T>,
        props: T,
        user_builder: impl for<'a, 'b> FnOnce(&'b mut DesktopConfig<'a>) -> &'b mut DesktopConfig<'a>,
    ) -> anyhow::Result<()> {
        Self::run_with_edits(root, props, user_builder, None)
    }

    pub fn run_with_edits(
        root: FC<T>,
        props: T,
        user_builder: impl for<'a, 'b> FnOnce(&'a mut DesktopConfig<'b>) -> &'a mut DesktopConfig<'b>,
        redits: Option<Vec<DomEdit<'static>>>,
    ) -> anyhow::Result<()> {
        log::info!("hello edits");
        let event_loop = EventLoop::new();

        let mut cfg = DesktopConfig::new();
        user_builder(&mut cfg);

        let DesktopConfig {
            window,
            manual_edits,
            pre_rendered,
        } = cfg;

        let window = window.build(&event_loop)?;

        let mut vir = VirtualDom::new_with_props(root, props);

        let channel = vir.get_event_sender();
        struct WebviewBridge {}
        // impl RealDom for WebviewBridge {
        //     fn raw_node_as_any(&self) -> &mut dyn std::any::Any {
        //         todo!()
        //     }

        //     fn must_commit(&self) -> bool {
        //         false
        //     }

        //     fn commit_edits<'a>(&mut self, edits: &mut Vec<DomEdit<'a>>) {}

        //     fn wait_until_ready<'s>(
        //         &'s mut self,
        //     ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 's>> {
        //         //
        //         Box::pin(async {
        //             //
        //         })
        //     }
        // }

        let mut real_dom = WebviewBridge {};
        // async_std::task::spawn_local(vir.run(&mut real_dom));

        // todo: combine these or something
        let vdom = Arc::new(RwLock::new(vir));
        // let registry = Arc::new(RwLock::new(Some(WebviewRegistry::new())));

        let webview = WebViewBuilder::new(window)?
            .with_url(&format!("data:text/html,{}", HTML_CONTENT))?
            // .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
            //     match req.method.as_str() {
            //         "initiate" => {
            //             let edits = if let Some(edits) = &redits {
            //                 serde_json::to_value(edits).unwrap()
            //             } else {
            //                 let mut lock = vdom.write().unwrap();
            //                 // let mut reg_lock = registry.write().unwrap();
            //                 // Create the thin wrapper around the registry to collect the edits into
            //                 let mut real = dom::WebviewDom::new();
            //                 let pre = pre_rendered.clone();
            //                 let response = match pre {
            //                     Some(content) => {
            //                         lock.rebuild_in_place().unwrap();
            //                         Response {
            //                             edits: Vec::new(),
            //                             pre_rendered: Some(content),
            //                         }
            //                     }
            //                     None => {
            //                         //
            //                         let edits = {
            //                             // let mut edits = Vec::new();
            //                             todo!()
            //                             // lock.rebuild(&mut real, &mut edits).unwrap();
            //                             // edits
            //                         };
            //                         Response {
            //                             edits,
            //                             pre_rendered: None,
            //                         }
            //                     }
            //                 };
            //                 serde_json::to_value(&response).unwrap()
            //             };
            //             // Return the edits into the webview runtime
            //             Some(RpcResponse::new_result(req.id.take(), Some(edits)))
            //         }
            //         "user_event" => {
            //             log::debug!("User event received");
            //             // let registry = registry.clone();
            //             let vdom = vdom.clone();
            //             let response = async_std::task::block_on(async move {
            //                 let mut lock = vdom.write().unwrap();
            //                 // let mut reg_lock = registry.write().unwrap();
            //                 // a deserialized event
            //                 let data = req.params.unwrap();
            //                 log::debug!("Data: {:#?}", data);
            //                 let event = trigger_from_serialized(data);
            //                 // lock.queue_event(event);
            //                 // Create the thin wrapper around the registry to collect the edits into
            //                 let mut real = dom::WebviewDom::new();
            //                 // Serialize the edit stream
            //                 //
            //                 let mut edits = Vec::new();
            //                 // lock.run(&mut real, &mut edits)
            //                 //     .await
            //                 //     .expect("failed to progress");
            //                 let response = Response {
            //                     edits,
            //                     pre_rendered: None,
            //                 };
            //                 let response = serde_json::to_value(&response).unwrap();
            //                 // Give back the registry into its slot
            //                 // *reg_lock = Some(real.consume());
            //                 // Return the edits into the webview runtime
            //                 Some(RpcResponse::new_result(req.id.take(), Some(response)))
            //             });
            //             response
            //             // spawn a task to clean up the garbage
            //         }
            //         _ => todo!("this message failed"),
            //     }
            // })
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
    }

    /// Create a new text renderer from an existing Virtual DOM.
    /// This will progress the existing VDom's events to completion.
    pub fn from_vdom() -> Self {
        todo!()
    }

    /// Pass new args to the root function
    pub fn update(&mut self, _new_val: T) {
        todo!()
    }

    /// Modify the root function in place, forcing a re-render regardless if the props changed
    pub fn update_mut(&mut self, _modifier: impl Fn(&mut T)) {
        todo!()
    }
}
