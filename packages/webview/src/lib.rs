use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use dioxus_core::*;

use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop};
use wry::application::window::Fullscreen;
use wry::webview::WebViewBuilder;
use wry::{
    application::window::{Window, WindowBuilder},
    webview::{RpcRequest, RpcResponse},
};

mod dom;
mod escape;

static HTML_CONTENT: &'static str = include_str!("./index.html");

pub fn launch(
    root: FC<()>,
    builder: impl FnOnce(WindowBuilder) -> WindowBuilder,
) -> anyhow::Result<()> {
    launch_with_props(root, (), builder)
}
pub fn launch_with_props<P: Properties + 'static>(
    root: FC<P>,
    props: P,
    builder: impl FnOnce(WindowBuilder) -> WindowBuilder,
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
    Initialize {
        //
        edits: Vec<DomEdit<'a>>,
    },
}

impl<T: Properties + 'static> WebviewRenderer<T> {
    pub fn run(
        root: FC<T>,
        props: T,
        user_builder: impl FnOnce(WindowBuilder) -> WindowBuilder,
    ) -> anyhow::Result<()> {
        Self::run_with_edits(root, props, user_builder, None)
    }

    pub fn run_with_edits(
        root: FC<T>,
        props: T,
        user_builder: impl FnOnce(WindowBuilder) -> WindowBuilder,
        redits: Option<Vec<DomEdit<'static>>>,
    ) -> anyhow::Result<()> {
        let event_loop = EventLoop::new();

        let window = user_builder(WindowBuilder::new()).build(&event_loop)?;

        let vir = VirtualDom::new_with_props(root, props);

        // todo: combine these or something
        let vdom = Arc::new(RwLock::new(vir));
        let registry = Arc::new(RwLock::new(Some(WebviewRegistry::new())));

        let webview = WebViewBuilder::new(window)?
            .with_url(&format!("data:text/html,{}", HTML_CONTENT))?
            .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
                match req.method.as_str() {
                    "initiate" => {
                        let edits = if let Some(edits) = &redits {
                            serde_json::to_value(edits).unwrap()
                        } else {
                            let mut lock = vdom.write().unwrap();
                            let mut reg_lock = registry.write().unwrap();

                            // Create the thin wrapper around the registry to collect the edits into
                            let mut real = dom::WebviewDom::new(reg_lock.take().unwrap());

                            // Serialize the edit stream
                            let edits = {
                                lock.rebuild(&mut real).unwrap();
                                serde_json::to_value(&real.edits).unwrap()
                            };

                            // Give back the registry into its slot
                            *reg_lock = Some(real.consume());
                            edits
                        };

                        // Return the edits into the webview runtime
                        Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                    }
                    "user_event" => {
                        let mut lock = vdom.write().unwrap();
                        let mut reg_lock = registry.write().unwrap();

                        // Create the thin wrapper around the registry to collect the edits into
                        let mut real = dom::WebviewDom::new(reg_lock.take().unwrap());

                        // Serialize the edit stream
                        let edits = {
                            lock.rebuild(&mut real).unwrap();
                            serde_json::to_value(&real.edits).unwrap()
                        };

                        // Give back the registry into its slot
                        *reg_lock = Some(real.consume());

                        // Return the edits into the webview runtime
                        Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                    }
                    _ => todo!("this message failed"),
                }
            })
            .build()?;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, .. } => {
                    //
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
                _ => {
                    // let _ = webview.resize();
                }
            }
        });
    }

    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    // pub fn new(root: FC<T>, builder: impl FnOnce() -> WVResult<WebView<'static, ()>>) -> Self {
    //     Self { root }
    // }

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

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dom::WebviewRegistry;

#[derive(Debug, Serialize, Deserialize)]
struct MessageParameters {
    message: String,
}

fn HANDLER(window: &Window, mut req: RpcRequest) -> Option<RpcResponse> {
    let mut response = None;
    if &req.method == "fullscreen" {
        if let Some(params) = req.params.take() {
            if let Ok(mut args) = serde_json::from_value::<Vec<bool>>(params) {
                if !args.is_empty() {
                    if args.swap_remove(0) {
                        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    } else {
                        window.set_fullscreen(None);
                    }
                };
                response = Some(RpcResponse::new_result(req.id.take(), None));
            }
        }
    } else if &req.method == "send-parameters" {
        if let Some(params) = req.params.take() {
            if let Ok(mut args) = serde_json::from_value::<Vec<MessageParameters>>(params) {
                let result = if !args.is_empty() {
                    let msg = args.swap_remove(0);
                    Some(Value::String(format!("Hello, {}!", msg.message)))
                } else {
                    // NOTE: in the real-world we should send an error response here!
                    None
                };
                // Must always send a response as this is a `call()`
                response = Some(RpcResponse::new_result(req.id.take(), result));
            }
        }
    }

    response
}
