#![cfg_attr(docsrs, feature(doc_cfg))]

//! A native renderer for Dioxus.
//!
//! ## Feature flags
//!  - `default`: Enables the features listed below.
//!  - `accessibility`: Enables [`accesskit`] accessibility support.
//!  - `hot-reload`: Enables hot-reloading of Dioxus RSX.
//!  - `menu`: Enables the [`muda`] menubar.
//!  - `tracing`: Enables tracing support.

mod application;
mod documents;
mod stylo_to_winit;
mod waker;
mod window;

#[cfg(all(feature = "menu", not(any(target_os = "android", target_os = "ios"))))]
mod menu;

#[cfg(feature = "accessibility")]
mod accessibility;

use crate::application::Application;
pub use crate::documents::DioxusDocument;
pub use crate::waker::BlitzEvent;
use crate::waker::BlitzWindowEvent;
use crate::window::View;
pub use crate::window::WindowConfig;
use blitz_dom::net::Resource;
use blitz_dom::{DocumentLike, HtmlDocument};
use blitz_net::Provider;
use blitz_traits::net::SharedCallback;
use dioxus::prelude::{ComponentFunction, Element, VirtualDom};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use url::Url;
use winit::event_loop::EventLoopProxy;
use winit::window::WindowId;
use winit::{
    dpi::LogicalSize,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod exports {
    pub use dioxus;
}

#[derive(Default)]
pub struct Config {
    pub stylesheets: Vec<String>,
    pub base_url: Option<String>,
}

/// Launch an interactive HTML/CSS renderer driven by the Dioxus virtualdom
pub fn launch(root: fn() -> Element) {
    launch_cfg(root, Config::default())
}

pub fn launch_cfg(root: fn() -> Element, cfg: Config) {
    launch_cfg_with_props(root, (), cfg)
}

// todo: props shouldn't have the clone bound - should try and match dioxus-desktop behavior
pub fn launch_cfg_with_props<P: Clone + 'static, M: 'static>(
    root: impl ComponentFunction<P, M>,
    props: P,
    _cfg: Config,
) {
    // Spin up the virtualdom
    // We're going to need to hit it with a special waker
    let vdom = VirtualDom::new_with_props(root, props);
    let document = DioxusDocument::new(vdom);

    // Turn on the runtime and enter it
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let _guard = rt.enter();
    launch_with_document(document, rt, None)
}

pub fn launch_url(url: &str) {
    const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:60.0) Gecko/20100101 Firefox/81.0";
    println!("{}", url);

    // Assert that url is valid
    let url = url.to_owned();
    Url::parse(&url).expect("Invalid url");

    let html = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .unwrap()
        .into_string()
        .unwrap();

    launch_static_html_cfg(
        &html,
        Config {
            stylesheets: Vec::new(),
            base_url: Some(url),
        },
    )
}

pub fn launch_static_html(html: &str) {
    launch_static_html_cfg(html, Config::default())
}

pub fn launch_static_html_cfg(html: &str, cfg: Config) {
    // Turn on the runtime and enter it
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let _guard = rt.enter();

    let net_callback = Arc::new(Callback::new());
    let net_provider = Arc::new(Provider::new(
        rt.handle().clone(),
        Arc::clone(&net_callback) as SharedCallback<Resource>,
    ));

    let document = HtmlDocument::from_html(html, cfg.base_url, cfg.stylesheets, net_provider, None);
    launch_with_document(document, rt, Some(net_callback));
}

pub fn launch_with_document(
    doc: impl DocumentLike,
    rt: Runtime,
    net_callback: Option<Arc<Callback>>,
) {
    let mut window_attrs = Window::default_attributes();
    if !cfg!(all(target_os = "android", target_os = "ios")) {
        window_attrs.inner_size = Some(
            LogicalSize {
                width: 800.,
                height: 600.,
            }
            .into(),
        );
    }
    let window = WindowConfig::new(doc, net_callback);

    launch_with_window(window, rt)
}

fn launch_with_window<Doc: DocumentLike + 'static>(window: WindowConfig<Doc>, rt: Runtime) {
    // Build an event loop for the application
    let mut ev_builder = EventLoop::<BlitzEvent>::with_user_event();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        ev_builder.with_android_app(current_android_app());
    }
    let event_loop = ev_builder.build().unwrap();
    let proxy = event_loop.create_proxy();
    event_loop.set_control_flow(ControlFlow::Wait);

    // Setup hot-reloading if enabled.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    {
        if let Some(endpoint) = dioxus_cli_config::devserver_ws_endpoint() {
            let proxy = proxy.clone();
            dioxus_devtools::connect(endpoint, move |event| {
                let _ = proxy.send_event(BlitzEvent::DevserverEvent(event));
            })
        }
    }

    // Create application
    let mut application = Application::new(rt, proxy);
    application.add_window(window);

    // Run event loop
    event_loop.run_app(&mut application).unwrap()
}

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
#[cfg_attr(docsrs, doc(cfg(target_os = "android")))]
/// Set the current [`AndroidApp`](android_activity::AndroidApp).
pub fn set_android_app(app: android_activity::AndroidApp) {
    ANDROID_APP.set(app).unwrap()
}

#[cfg(target_os = "android")]
#[cfg_attr(docsrs, doc(cfg(target_os = "android")))]
/// Get the current [`AndroidApp`](android_activity::AndroidApp).
/// This will panic if the android activity has not been setup with [`set_android_app`].
pub fn current_android_app(app: android_activity::AndroidApp) -> AndroidApp {
    ANDROID_APP.get().unwrap().clone()
}

pub struct Callback(Mutex<CallbackInner>);
enum CallbackInner {
    Window(WindowId, EventLoopProxy<BlitzEvent>),
    Queue(Vec<Resource>),
}
impl Callback {
    pub fn new() -> Self {
        Default::default()
    }
    fn init(self: Arc<Self>, window_id: WindowId, proxy: &EventLoopProxy<BlitzEvent>) {
        let old = std::mem::replace(
            self.0.lock().unwrap().deref_mut(),
            CallbackInner::Window(window_id, proxy.clone()),
        );
        match old {
            CallbackInner::Window(..) => {}
            CallbackInner::Queue(mut queue) => queue
                .drain(..)
                .for_each(|res| Self::send_event(&window_id, proxy, res)),
        }
    }
    fn send_event(window_id: &WindowId, proxy: &EventLoopProxy<BlitzEvent>, data: Resource) {
        proxy
            .send_event(BlitzEvent::Window {
                window_id: *window_id,
                data: BlitzWindowEvent::ResourceLoad(data),
            })
            .unwrap()
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self(Mutex::new(CallbackInner::Queue(Vec::new())))
    }
}

impl blitz_traits::net::Callback for Callback {
    type Data = Resource;
    fn call(&self, data: Self::Data) {
        match self.0.lock().unwrap().deref_mut() {
            CallbackInner::Window(wid, proxy) => Self::send_event(wid, proxy, data),
            CallbackInner::Queue(queue) => queue.push(data),
        }
    }
}
