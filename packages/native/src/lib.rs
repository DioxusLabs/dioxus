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
mod events;
mod mutation_writer;

use blitz_dom::{ns, Atom, QualName};
pub use dioxus_application::{DioxusNativeApplication, DioxusNativeEvent};
pub use dioxus_document::DioxusDocument;

use blitz_shell::{create_default_event_loop, BlitzShellEvent, Config, WindowConfig};
use dioxus_core::{ComponentFunction, Element, VirtualDom};
use std::any::Any;

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
    _cfg: Vec<Box<dyn Any>>,
) {
    let _cfg = _cfg
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();
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

    // Create application
    let mut application = DioxusNativeApplication::new(event_loop.create_proxy(), vdom);

    // Run event loop
    event_loop.run_app(&mut application).unwrap();
}

pub(crate) fn qual_name(local_name: &str, namespace: Option<&str>) -> QualName {
    QualName {
        prefix: None,
        ns: namespace.map(Atom::from).unwrap_or(ns!(html)),
        local: Atom::from(local_name),
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
