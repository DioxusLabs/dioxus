use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

use dioxus_core::virtual_dom::VirtualDom;
use dioxus_core::{prelude::*, serialize::DomEdit};
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
        use wry::{
            application::{
                event::{Event, StartCause, WindowEvent},
                event_loop::{ControlFlow, EventLoop},
                window::WindowBuilder,
            },
            webview::WebViewBuilder,
        };

        let event_loop = EventLoop::new();

        let window = user_builder(WindowBuilder::new()).build(&event_loop)?;

        let vir = VirtualDom::new_with_props(root, props);

        // todo: combine these or something
        let vdom = Arc::new(RwLock::new(vir));
        let registry = Arc::new(RwLock::new(Some(WebviewRegistry::new())));

        let webview = WebViewBuilder::new(window)?
            .with_url(&format!("data:text/html,{}", HTML_CONTENT))?
            .with_rpc_handler(move |window: &Window, mut req: RpcRequest| {
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

        // let mut view = web_view::builder()
        //     .invoke_handler(|view, arg| {
        //         let handle = view.handle();
        //         sender
        //             .send(InnerEvent::Initiate(handle))
        //             .expect("should not fail");

        //         Ok(())
        //     })
        //     .content(web_view::Content::Html(HTML_CONTENT))
        //     .user_data(())
        //     .title(title)
        //     .size(width, height)
        //     .resizable(resizable)
        //     .debug(debug)
        //     .frameless(frameless)
        //     .visible(visible)
        //     .min_size(min_width, min_height)
        //     .build()
        //     .unwrap();
        // loop {
        //     view.step()
        //         .expect("should not fail")
        //         .expect("should not fail");
        //     std::thread::sleep(std::time::Duration::from_millis(15));

        //     if let Ok(event) = receiver.try_recv() {
        //         if let InnerEvent::Initiate(handle) = event {
        //             let editlist = ref_edits.clone();
        //             handle
        //                 .dispatch(move |view| {
        //                     let escaped = escape(&editlist);
        //                     view.eval(&format!("EditListReceived({});", escaped))
        //                 })
        //                 .expect("Dispatch failed");
        //         }
        //     }
        // }
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
    pub fn update(&mut self, new_val: T) {
        todo!()
    }

    /// Modify the root function in place, forcing a re-render regardless if the props changed
    pub fn update_mut(&mut self, modifier: impl Fn(&mut T)) {
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
    use wry::{
        application::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::{Fullscreen, Window, WindowBuilder},
        },
        webview::{RpcRequest, RpcResponse, WebViewBuilder},
    };

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

pub struct DioxusWebviewBuilder<'a> {
    pub(crate) title: &'a str,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) resizable: bool,
    pub(crate) debug: bool,
    pub(crate) frameless: bool,
    pub(crate) visible: bool,
    pub(crate) min_width: i32,
    pub(crate) min_height: i32,
}
impl<'a> DioxusWebviewBuilder<'a> {
    fn new() -> Self {
        #[cfg(debug_assertions)]
        let debug = true;
        #[cfg(not(debug_assertions))]
        let debug = false;

        DioxusWebviewBuilder {
            title: "Application",
            width: 800,
            height: 600,
            resizable: true,
            debug,
            frameless: false,
            visible: true,
            min_width: 300,
            min_height: 300,
        }
    }
    /// Sets the title of the WebView window.
    ///
    /// Defaults to `"Application"`.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Sets the size of the WebView window.
    ///
    /// Defaults to 800 x 600.
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the resizability of the WebView window. If set to false, the window cannot be resized.
    ///
    /// Defaults to `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Enables or disables debug mode.
    ///
    /// Defaults to `true` for debug builds, `false` for release builds.
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
    /// The window crated will be frameless
    ///
    /// defaults to `false`
    pub fn frameless(mut self, frameless: bool) -> Self {
        self.frameless = frameless;
        self
    }

    /// Set the visibility of the WebView window.
    ///
    /// defaults to `true`
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets the minimum size of the WebView window.
    ///
    /// Defaults to 300 x 300.
    pub fn min_size(mut self, width: i32, height: i32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}
