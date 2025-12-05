#![cfg_attr(docsrs, feature(doc_cfg))]

//! A native renderer for Dioxus.
//!
//! ## Feature flags
//!  - `default`: Enables the features listed below.
//!  - `accessibility`: Enables [`accesskit`](https://docs.rs/accesskit/latest/accesskit/) accessibility support.
//!  - `hot-reload`: Enables hot-reloading of Dioxus RSX.
//!  - `menu`: Enables the [`muda`](https://docs.rs/muda/latest/muda/) menubar.
//!  - `tracing`: Enables tracing support.

mod assets;
mod contexts;
mod dioxus_application;
mod dioxus_renderer;
mod link_handler;

#[doc(inline)]
pub use dioxus_native_dom::*;

pub use anyrender_vello::{CustomPaintCtx, CustomPaintSource, DeviceHandle, TextureHandle};
use assets::DioxusNativeNetProvider;
pub use dioxus_application::{DioxusNativeApplication, DioxusNativeEvent};
pub use dioxus_renderer::{DioxusNativeWindowRenderer, Features, Limits};

#[cfg(not(all(target_os = "ios", target_abi = "sim")))]
pub use dioxus_renderer::use_wgpu;

use blitz_shell::{create_default_event_loop, BlitzShellEvent, Config, WindowConfig};
use dioxus_core::{ComponentFunction, Element, VirtualDom};
use link_handler::DioxusNativeNavigationProvider;
use std::any::Any;
use std::sync::Arc;
use winit::window::WindowAttributes;

/// Launch an interactive HTML/CSS renderer driven by the Dioxus virtualdom
pub fn launch(app: fn() -> Element) {
    launch_cfg(app, vec![], vec![])
}

pub fn launch_cfg(
    app: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    cfg: Vec<Box<dyn Any>>,
) {
    launch_cfg_with_props(app, (), contexts, cfg)
}

// todo: props shouldn't have the clone bound - should try and match dioxus-desktop behavior
pub fn launch_cfg_with_props<P: Clone + 'static, M: 'static>(
    app: impl ComponentFunction<P, M>,
    props: P,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    configs: Vec<Box<dyn Any>>,
) {
    // Macro to attempt to downcast a type out of a Box<dyn Any>
    macro_rules! try_read_config {
        ($input:ident, $store:ident, $kind:ty) => {
            // Try to downcast the Box<dyn Any> to type $kind
            match $input.downcast::<$kind>() {
                // If the type matches then write downcast value to variable $store
                Ok(value) => {
                    $store = Some(*value);
                    continue;
                }
                // Else extract the original Box<dyn Any> value out of the error type
                // and return it so that we can try again with a different type.
                Err(cfg) => cfg,
            }
        };
    }

    // Read config values
    let mut features = None;
    let mut limits = None;
    let mut window_attributes = None;
    let mut _config = None;
    for mut cfg in configs {
        cfg = try_read_config!(cfg, features, Features);
        cfg = try_read_config!(cfg, limits, Limits);
        cfg = try_read_config!(cfg, window_attributes, WindowAttributes);
        cfg = try_read_config!(cfg, _config, Config);
        let _ = cfg;
    }

    let event_loop = create_default_event_loop::<BlitzShellEvent>();

    // Turn on the runtime and enter it
    #[cfg(feature = "net")]
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    #[cfg(feature = "net")]
    let _guard = rt.enter();

    // Setup hot-reloading if enabled.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    {
        let proxy = event_loop.create_proxy();
        dioxus_devtools::connect(move |event| {
            let dxn_event = DioxusNativeEvent::DevserverEvent(event);
            let _ = proxy.send_event(BlitzShellEvent::embedder_event(dxn_event));
        })
    }

    // Spin up the virtualdom
    // We're going to need to hit it with a special waker
    // Note that we are delaying the initialization of window-specific contexts (net provider, document, etc)
    let mut vdom = VirtualDom::new_with_props(app, props);

    // Add contexts
    for context in contexts {
        vdom.insert_any_root_context(context());
    }

    let net_provider = Some(DioxusNativeNetProvider::shared(event_loop.create_proxy()));

    #[cfg(feature = "html")]
    let html_parser_provider = Some(Arc::new(blitz_html::HtmlProvider) as _);
    #[cfg(not(feature = "html"))]
    let html_parser_provider = None;

    let navigation_provider = Some(Arc::new(DioxusNativeNavigationProvider) as _);

    // Create document + window from the baked virtualdom
    let doc = DioxusDocument::new(
        vdom,
        DocumentConfig {
            net_provider,
            html_parser_provider,
            navigation_provider,
            ..Default::default()
        },
    );
    #[cfg(not(all(target_os = "ios", target_abi = "sim")))]
    let renderer = DioxusNativeWindowRenderer::with_features_and_limits(features, limits);
    #[cfg(all(target_os = "ios", target_abi = "sim"))]
    let renderer = DioxusNativeWindowRenderer::new();
    let config = WindowConfig::with_attributes(
        Box::new(doc) as _,
        renderer.clone(),
        window_attributes.unwrap_or_default(),
    );

    // Create application
    let mut application = DioxusNativeApplication::new(event_loop.create_proxy(), config);

    // Run event loop
    event_loop.run_app(&mut application).unwrap();
}
