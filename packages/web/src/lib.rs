#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

//! Dioxus WebSys
//!
//! ## Overview
//! ------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using WebSys. This web render for
//! Dioxus is one of the more advanced renderers, supporting:
//! - idle work
//! - animations
//! - jank-free rendering
//! - controlled components
//! - hydration
//! - and more.
//!
//! The actual implementation is farily thin, with the heavy lifting happening inside the Dioxus Core crate.
//!
//! To purview the examples, check of the root Dioxus crate - the examples in this crate are mostly meant to provide
//! validation of websys-specific features and not the general use of Dioxus.

pub use crate::cfg::Config;
use crate::hydration::SuspenseMessage;
use dioxus_core::VirtualDom;
use dom::WebsysDom;
use futures_util::{pin_mut, select, FutureExt, StreamExt};

mod cfg;
mod dom;
mod event;
pub mod launch;
mod mutations;
use event::*;

#[cfg(feature = "document")]
mod document;

#[cfg(feature = "document")]
pub use document::WebDocument;

#[cfg(all(feature = "devtools", debug_assertions))]
mod devtools;

mod hydration;

#[allow(unused)]
pub use hydration::*;

/// Runs the app as a future that can be scheduled around the main thread.
///
/// Polls futures internal to the VirtualDOM, hence the async nature of this function.
///
/// # Example
///
/// ```ignore, rust
/// let app_fut = dioxus_web::run_with_props(App, RootProps { name: String::from("foo") });
/// wasm_bindgen_futures::spawn_local(app_fut);
/// ```
pub async fn run(mut virtual_dom: VirtualDom, web_config: Config) -> ! {
    tracing::info!("Starting up");

    #[cfg(feature = "document")]
    virtual_dom.in_runtime(document::init_document);

    #[cfg(feature = "panic_hook")]
    if web_config.default_panic_hook {
        console_error_panic_hook::set_once();
    }

    #[cfg(all(feature = "devtools", debug_assertions))]
    let mut hotreload_rx = devtools::init();

    let runtime = virtual_dom.runtime();

    let should_hydrate = web_config.hydrate;

    let mut websys_dom = WebsysDom::new(web_config, runtime);

    let mut hydration_receiver: Option<futures_channel::mpsc::UnboundedReceiver<SuspenseMessage>> =
        None;

    if should_hydrate {
        #[cfg(feature = "hydrate")]
        {
            websys_dom.skip_mutations = true;
            // Get the initial hydration data from the client
            #[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
                export function get_initial_hydration_data() {
                    const decoded = atob(window.initial_dioxus_hydration_data);
                    return Uint8Array.from(decoded, (c) => c.charCodeAt(0))
                }
            "#)]
            extern "C" {
                fn get_initial_hydration_data() -> js_sys::Uint8Array;
            }
            let hydration_data = get_initial_hydration_data().to_vec();
            let server_data = HTMLDataCursor::from_serialized(&hydration_data);
            // If the server serialized an error into the root suspense boundary, throw it into the root scope
            if let Some(error) = server_data.error() {
                virtual_dom.in_runtime(|| dioxus_core::ScopeId::APP.throw_error(error));
            }
            with_server_data(server_data, || {
                virtual_dom.rebuild(&mut websys_dom);
            });
            websys_dom.skip_mutations = false;

            let rx = websys_dom.rehydrate(&virtual_dom).unwrap();
            hydration_receiver = Some(rx);
        }
        #[cfg(not(feature = "hydrate"))]
        {
            panic!("Hydration is not enabled. Please enable the `hydrate` feature.");
        }
    } else {
        virtual_dom.rebuild(&mut websys_dom);

        websys_dom.flush_edits();
    }

    // the mutations come back with nothing - we need to actually mount them
    websys_dom.mount();

    loop {
        // if virtual dom has nothing, wait for it to have something before requesting idle time
        // if there is work then this future resolves immediately.
        #[cfg(all(feature = "devtools", debug_assertions))]
        let template;
        #[allow(unused)]
        let mut hydration_work: Option<SuspenseMessage> = None;

        {
            let work = virtual_dom.wait_for_work().fuse();
            pin_mut!(work);

            let mut hydration_receiver_iter = futures_util::stream::iter(&mut hydration_receiver)
                .fuse()
                .flatten();
            let mut rx_hydration = hydration_receiver_iter.select_next_some();

            #[cfg(all(feature = "devtools", debug_assertions))]
            #[allow(unused)]
            {
                let mut devtools_next = hotreload_rx.select_next_some();
                select! {
                    _ = work => {
                        template = None;
                    },
                    new_template = devtools_next => {
                        template = Some(new_template);
                    },
                    hydration_data = rx_hydration => {
                        template = None;
                        #[cfg(feature = "hydrate")]
                        {
                            hydration_work = Some(hydration_data);
                        }
                    },
                }
            }

            #[cfg(not(all(feature = "devtools", debug_assertions)))]
            #[allow(unused)]
            {
                select! {
                    _ = work => {},
                    hyd = rx_hydration => {
                        #[cfg(feature = "hydrate")]
                        {
                            hydration_work = Some(hyd);
                        }
                    }
                }
            }
        }

        #[cfg(all(feature = "devtools", debug_assertions))]
        if let Some(hr_msg) = template {
            // Replace all templates
            dioxus_devtools::apply_changes(&virtual_dom, &hr_msg);

            if !hr_msg.assets.is_empty() {
                crate::devtools::invalidate_browser_asset_cache();
            }
        }

        #[cfg(feature = "hydrate")]
        if let Some(hydration_data) = hydration_work {
            websys_dom.rehydrate_streaming(hydration_data, &mut virtual_dom);
        }

        // run the virtualdom work phase until the frame deadline is reached
        virtual_dom.render_immediate(&mut websys_dom);

        // Flush all pending edits to the dom in one swoop
        websys_dom.flush_edits();
    }
}
