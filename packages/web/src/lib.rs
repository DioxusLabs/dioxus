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

mod events;
pub mod launch;
mod mutations;
pub use events::*;

#[cfg(feature = "document")]
mod document;
#[cfg(feature = "file_engine")]
mod file_engine;
#[cfg(feature = "document")]
mod history;
#[cfg(feature = "document")]
pub use document::WebDocument;
#[cfg(feature = "file_engine")]
pub use file_engine::*;

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
    #[cfg(feature = "document")]
    virtual_dom.in_runtime(document::init_document);

    let runtime = virtual_dom.runtime();

    #[cfg(all(feature = "devtools", debug_assertions))]
    let mut hotreload_rx = devtools::init(runtime.clone());

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
                export function get_initial_hydration_debug_types() {
                    return window.initial_dioxus_hydration_debug_types;
                }
                export function get_initial_hydration_debug_locations() {
                    return window.initial_dioxus_hydration_debug_locations;
                }
            "#)]
            extern "C" {
                fn get_initial_hydration_data() -> js_sys::Uint8Array;
                fn get_initial_hydration_debug_types() -> Option<Vec<String>>;
                fn get_initial_hydration_debug_locations() -> Option<Vec<String>>;
            }
            let hydration_data = get_initial_hydration_data().to_vec();

            // If we are running in debug mode, also get the debug types and locations
            #[cfg(debug_assertions)]
            let debug_types = get_initial_hydration_debug_types();
            #[cfg(not(debug_assertions))]
            let debug_types = None;
            #[cfg(debug_assertions)]
            let debug_locations = get_initial_hydration_debug_locations();
            #[cfg(not(debug_assertions))]
            let debug_locations = None;

            let server_data =
                HTMLDataCursor::from_serialized(&hydration_data, debug_types, debug_locations);
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

            #[cfg(feature = "mounted")]
            {
                // Flush any mounted events that were queued up while hydrating
                websys_dom.flush_queued_mounted_events();
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            panic!("Hydration is not enabled. Please enable the `hydrate` feature.");
        }
    } else {
        virtual_dom.rebuild(&mut websys_dom);

        websys_dom.flush_edits();
    }

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

        // Todo: This is currently disabled because it has a negative impact on response times for events but it could be re-enabled for tasks
        // Jank free rendering
        //
        // 1. wait for the browser to give us "idle" time
        // 2. During idle time, diff the dom
        // 3. Stop diffing if the deadline is exceeded
        // 4. Wait for the animation frame to patch the dom

        // wait for the mainthread to schedule us in
        // let deadline = work_loop.wait_for_idle_time().await;

        // run the virtualdom work phase until the frame deadline is reached
        virtual_dom.render_immediate(&mut websys_dom);

        // wait for the animation frame to fire so we can apply our changes
        // work_loop.wait_for_raf().await;

        websys_dom.flush_edits();
    }
}
