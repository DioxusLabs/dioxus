use dioxus_core::*;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop};
use wry::application::window::Fullscreen;
use wry::application::{
    dpi::LogicalSize,
    event::StartCause,
    // platform::ios::{ScreenEdge, WindowBuilderExtIOS, WindowExtIOS},
    // platform::ios::{ScreenEdge, WindowBuilderExtIOS, WindowExtIOS},
};
use wry::webview::WebViewBuilder;
use wry::{
    application::window::{Window, WindowBuilder},
    webview::{RpcRequest, RpcResponse},
};
mod dom;
use dom::*;

fn init_logging() {
    simple_logger::SimpleLogger::new().init().unwrap();
}

static HTML_CONTENT: &'static str = include_str!("../../desktop/src/index.html");

pub fn launch(root: FC<()>, builder: fn(WindowBuilder) -> WindowBuilder) -> anyhow::Result<()> {
    launch_with_props(root, (), builder)
}
pub fn launch_with_props<P: 'static + Send>(
    root: FC<P>,
    props: P,
    builder: fn(WindowBuilder) -> WindowBuilder,
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

impl<T: 'static + Send> WebviewRenderer<T> {
    pub fn run(
        root: FC<T>,
        props: T,
        user_builder: fn(WindowBuilder) -> WindowBuilder,
    ) -> anyhow::Result<()> {
        Self::run_with_edits(root, props, user_builder, None)
    }

    pub fn run_with_edits(
        root: FC<T>,
        props: T,
        user_builder: fn(WindowBuilder) -> WindowBuilder,
        redits: Option<Vec<DomEdit<'static>>>,
    ) -> anyhow::Result<()> {
        // pub fn run_with_edits(
        //     root: FC<T>,
        //     props: T,
        //     user_builder: fn(WindowBuilder) -> WindowBuilder,
        //     redits: Option<Vec<DomEdit<'static>>>,
        // ) -> anyhow::Result<()> {
        let mut weviews = HashMap::new();

        let vir = VirtualDom::new_with_props(root, props);

        let vdom = Arc::new(RwLock::new(vir));

        let event_loop = EventLoop::new();
        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Init");

                    let window = user_builder(WindowBuilder::new())
                        .build(&event_loop)
                        .unwrap();

                    let registry = Arc::new(RwLock::new(Some(WebviewRegistry::new())));

                    let window = WindowBuilder::new().build(&event_loop).unwrap();
                    let window_id = window.id();

                    let vdom = vdom.clone();
                    let weview = WebViewBuilder::new(window)
                        .unwrap()
                        // .with_visible(false)
                        // .with_transparent(true)
                        .with_url(&format!("data:text/html,{}", HTML_CONTENT))
                        .unwrap()
                        .with_rpc_handler(move |_window: &Window, mut req: RpcRequest| {
                            match req.method.as_str() {
                                "initiate" => {
                                    // let edits = if let Some(edits) = &redits {
                                    //     serde_json::to_value(edits).unwrap()
                                    // } else
                                    let edits = {
                                        let mut lock = vdom.write().unwrap();
                                        let mut reg_lock = registry.write().unwrap();

                                        // Create the thin wrapper around the registry to collect the edits into
                                        let mut real =
                                            dom::WebviewDom::new(reg_lock.take().unwrap());

                                        // Serialize the edit stream
                                        let edits = {
                                            let mut edits = Vec::<DomEdit>::new();
                                            // lock.rebuild(&mut edits).unwrap();
                                            serde_json::to_value(edits).unwrap()
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
                                        let mut edits = Vec::<DomEdit>::new();
                                        // lock.rebuild(&mut edits).unwrap();
                                        serde_json::to_value(edits).unwrap()
                                    };

                                    // Give back the registry into its slot
                                    *reg_lock = Some(real.consume());

                                    // Return the edits into the webview runtime
                                    Some(RpcResponse::new_result(req.id.take(), Some(edits)))
                                }
                                _ => todo!("this message failed"),
                            }
                        })
                        .build()
                        .unwrap();

                    // let weview = WebViewBuilder::new(window)
                    //     .unwrap()
                    //     .with_url("https://tauri.studio")
                    //     .unwrap()
                    //     .build()
                    //     .unwrap();
                    weviews.insert(window_id, weview);
                }
                Event::Resumed => {
                    println!("applicationDidBecomeActive");
                }
                Event::Suspended => {
                    println!("applicationWillResignActive");
                }
                Event::LoopDestroyed => {
                    println!("applicationWillTerminate");
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Touch(touch),
                    ..
                } => {
                    println!("touch on {:?} {:?}", window_id, touch);
                }
                _ => {}
            }
        });
    }
}
// brad johnson go chat

fn main() {
    init_logging();
    let event_loop = EventLoop::new();

    let mut weviews = HashMap::new();

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("Init");

                let window = WindowBuilder::new().build(&event_loop).unwrap();
                let window_id = window.id();

                let weview = WebViewBuilder::new(window)
                    .unwrap()
                    .with_url("https://tauri.studio")
                    .unwrap()
                    .build()
                    .unwrap();
                weviews.insert(window_id, weview);
            }
            Event::Resumed => {
                println!("applicationDidBecomeActive");
            }
            Event::Suspended => {
                println!("applicationWillResignActive");
            }
            Event::LoopDestroyed => {
                println!("applicationWillTerminate");
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Touch(touch),
                ..
            } => {
                println!("touch on {:?} {:?}", window_id, touch);
            }
            _ => {}
        }
    });
}
