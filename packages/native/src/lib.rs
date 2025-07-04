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
mod dioxus_document;
mod dioxus_renderer;
mod events;
mod mutation_writer;

pub use anyrender_vello::{
    wgpu_context::DeviceHandle, CustomPaintCtx, CustomPaintSource, TextureHandle,
};
use assets::DioxusNativeNetProvider;
use blitz_dom::{ns, LocalName, Namespace, QualName};
pub use dioxus_application::{DioxusNativeApplication, DioxusNativeEvent};
pub use dioxus_document::DioxusDocument;
pub use dioxus_renderer::{use_wgpu, DioxusNativeWindowRenderer, Features, Limits};

use blitz_shell::{create_default_event_loop, BlitzShellEvent, Config, WindowConfig};
use dioxus_core::{ComponentFunction, Element, VirtualDom};
use std::any::Any;
use winit::window::WindowAttributes;

type NodeId = usize;

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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
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

    #[cfg(feature = "net")]
    let net_provider = {
        let proxy = event_loop.create_proxy();
        let net_provider = DioxusNativeNetProvider::shared(proxy);
        Some(net_provider)
    };

    #[cfg(not(feature = "net"))]
    let net_provider = None;

    // Create document + window from the baked virtualdom
    let doc = DioxusDocument::new(vdom, net_provider);
    let renderer = DioxusNativeWindowRenderer::with_features_and_limits(features, limits);
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

pub(crate) fn qual_name(local_name: &str, namespace: Option<&str>) -> QualName {
    QualName {
        prefix: None,
        ns: namespace.map(Namespace::from).unwrap_or(ns!(html)),
        local: LocalName::from(local_name),
    }
}

// Syntax sugar to make tracing calls less noisy in function below
macro_rules! trace {
    ($pattern:literal) => {{
        #[cfg(feature = "tracing")]
        tracing::info!($pattern);
    }};
    ($pattern:literal, $item1:expr) => {{
        #[cfg(feature = "tracing")]
        tracing::info!($pattern, $item1);
    }};
    ($pattern:literal, $item1:expr, $item2:expr) => {{
        #[cfg(feature = "tracing")]
        tracing::info!($pattern, $item1, $item2);
    }};
    ($pattern:literal, $item1:expr, $item2:expr, $item3:expr) => {{
        #[cfg(feature = "tracing")]
        tracing::info!($pattern, $item1, $item2);
    }};
    ($pattern:literal, $item1:expr, $item2:expr, $item3:expr, $item4:expr) => {{
        #[cfg(feature = "tracing")]
        tracing::info!($pattern, $item1, $item2, $item3, $item4);
    }};
}
pub(crate) use trace;
