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
use crate::window::View;
pub use crate::window::WindowConfig;
use blitz_dom::net::Resource;
use blitz_dom::{DocumentLike, HtmlDocument};
use blitz_net::Provider;
use blitz_traits::net::{NetCallback, SharedCallback};
use dioxus::prelude::{ComponentFunction, Element, VirtualDom};
use std::sync::Arc;
use tokio::runtime::Runtime;
use url::Url;
use winit::event_loop::EventLoopProxy;
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

/// Build an event loop for the application
pub fn create_default_event_loop<Event>() -> EventLoop<Event> {
    let mut ev_builder = EventLoop::<Event>::with_user_event();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        ev_builder.with_android_app(current_android_app());
    }

    let event_loop = ev_builder.build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
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
    // Turn on the runtime and enter it
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    let event_loop = create_default_event_loop::<BlitzEvent>();
    let proxy = event_loop.create_proxy();

    let net_callback = Arc::new(WinitNetCallback(proxy));
    let net_provider = Arc::new(Provider::new(
        rt.handle().clone(),
        Arc::clone(&net_callback) as SharedCallback<Resource>,
    ));

    // Spin up the virtualdom
    // We're going to need to hit it with a special waker
    let vdom = VirtualDom::new_with_props(root, props);
    let document = DioxusDocument::new(vdom, Some(net_provider));

    launch_with_document(document, rt, event_loop)
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

    let event_loop = create_default_event_loop::<BlitzEvent>();
    let proxy = event_loop.create_proxy();

    let net_callback = Arc::new(WinitNetCallback(proxy));
    let net_provider = Arc::new(Provider::new(
        rt.handle().clone(),
        Arc::clone(&net_callback) as SharedCallback<Resource>,
    ));

    let document = HtmlDocument::from_html(html, cfg.base_url, cfg.stylesheets, net_provider, None);
    launch_with_document(document, rt, event_loop);
}

pub fn launch_with_document(
    doc: impl DocumentLike,
    rt: Runtime,
    event_loop: EventLoop<BlitzEvent>,
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
    let window = WindowConfig::new(doc);

    launch_with_window(window, rt, event_loop)
}

fn launch_with_window<Doc: DocumentLike + 'static>(
    window: WindowConfig<Doc>,
    rt: Runtime,
    event_loop: EventLoop<BlitzEvent>,
) {
    // Setup hot-reloading if enabled.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    {
        if let Some(endpoint) = dioxus_cli_config::devserver_ws_endpoint() {
            let proxy = event_loop.create_proxy();
            dioxus_devtools::connect(endpoint, move |event| {
                let _ = proxy.send_event(BlitzEvent::DevserverEvent(event));
            })
        }
    }

    // Create application
    let mut application = Application::new(rt, event_loop.create_proxy());
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

/// A NetCallback that injects the fetched Resource into our winit event loop
pub struct WinitNetCallback(EventLoopProxy<BlitzEvent>);

impl WinitNetCallback {
    pub fn new(proxy: EventLoopProxy<BlitzEvent>) -> Self {
        Self(proxy)
    }
}
impl NetCallback for WinitNetCallback {
    type Data = Resource;
    fn call(&self, doc_id: usize, data: Self::Data) {
        self.0
            .send_event(BlitzEvent::ResourceLoad { doc_id, data })
            .unwrap()
    }
}
