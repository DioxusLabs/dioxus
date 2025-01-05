#![cfg_attr(docsrs, feature(doc_cfg))]

//! A native renderer for Dioxus.
//!
//! ## Feature flags
//!  - `default`: Enables the features listed below.
//!  - `accessibility`: Enables [`accesskit`] accessibility support.
//!  - `hot-reload`: Enables hot-reloading of Dioxus RSX.
//!  - `menu`: Enables the [`muda`] menubar.
//!  - `tracing`: Enables tracing support.

mod dioxus_application;
mod dioxus_document;
mod event;
mod event_handler;
mod keyboard_event;
mod mutation_writer;

pub use dioxus_application::DioxusNativeApplication;
pub use dioxus_document::DioxusDocument;
pub use event::DioxusNativeEvent;

use blitz_shell::{create_default_event_loop, BlitzShellEvent, Config, WindowConfig};
use dioxus_core::{ComponentFunction, Element, VirtualDom};

type NodeId = usize;

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
    let event_loop = create_default_event_loop::<BlitzShellEvent>();

    #[cfg(feature = "net")]
    let net_provider = {
        use blitz_net::Provider;
        use blitz_shell::BlitzShellNetCallback;

        // Turn on the runtime and enter it
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();

        let proxy = event_loop.create_proxy();
        let net_callback = BlitzShellNetCallback::shared(proxy);
        let net_provider = Provider::shared(net_callback);

        Some(net_provider)
    };

    #[cfg(not(feature = "net"))]
    let net_provider = None;

    // Spin up the virtualdom
    // We're going to need to hit it with a special waker
    let vdom = VirtualDom::new_with_props(root, props);
    let doc = DioxusDocument::new(vdom, net_provider);
    let window = WindowConfig::new(doc);

    // Setup hot-reloading if enabled.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    {
        use crate::event::DioxusNativeEvent;
        if let Some(endpoint) = dioxus_cli_config::devserver_ws_endpoint() {
            let proxy = event_loop.create_proxy();
            dioxus_devtools::connect(endpoint, move |event| {
                let dxn_event = DioxusNativeEvent::DevserverEvent(event);
                let _ = proxy.send_event(BlitzShellEvent::embedder_event(dxn_event));
            })
        }
    }

    // Create application
    let mut application = DioxusNativeApplication::new(event_loop.create_proxy());
    application.add_window(window);

    // Run event loop
    event_loop.run_app(&mut application).unwrap();
}
