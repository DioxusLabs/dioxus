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

use std::{panic, rc::Rc};

pub use crate::cfg::Config;
use crate::hydration::SuspenseMessage;
use dioxus_core::VirtualDom;
use futures_util::{pin_mut, select, FutureExt, StreamExt};

mod cfg;
mod dom;

mod event;
pub mod launch;
mod mutations;
pub use event::*;

#[cfg(feature = "document")]
mod document;
#[cfg(feature = "document")]
pub use document::WebDocument;

#[cfg(all(feature = "hot_reload", debug_assertions))]
mod hot_reload;

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
pub async fn run(virtual_dom: VirtualDom, web_config: Config) -> ! {
    tracing::info!("Starting up");

    let mut dom = virtual_dom;

    #[cfg(feature = "document")]
    dom.in_runtime(document::init_document);

    #[cfg(feature = "panic_hook")]
    if web_config.default_panic_hook {
        console_error_panic_hook::set_once();
    }

    #[cfg(all(feature = "hot_reload", debug_assertions))]
    let mut hotreload_rx = hot_reload::init();

    let (tx, mut rx) = futures_channel::mpsc::unbounded();

    let should_hydrate = web_config.hydrate;

    let mut websys_dom = dom::WebsysDom::new(web_config, tx);

    let mut hydration_receiver: Option<futures_channel::mpsc::UnboundedReceiver<SuspenseMessage>> =
        None;

    if should_hydrate {
        #[cfg(feature = "hydrate")]
        {
            websys_dom.only_write_templates = true;
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
            with_server_data(server_data, || {
                dom.rebuild(&mut websys_dom);
            });
            websys_dom.only_write_templates = false;

            let rx = websys_dom.rehydrate(&dom).unwrap();
            hydration_receiver = Some(rx);
        }
        #[cfg(not(feature = "hydrate"))]
        {
            panic!("Hydration is not enabled. Please enable the `hydrate` feature.");
        }
    } else {
        dom.rebuild(&mut websys_dom);

        websys_dom.flush_edits();
    }

    // the mutations come back with nothing - we need to actually mount them
    websys_dom.mount();

    loop {
        // if virtual dom has nothing, wait for it to have something before requesting idle time
        // if there is work then this future resolves immediately.
        let mut res;
        #[cfg(all(feature = "hot_reload", debug_assertions))]
        let template;
        #[allow(unused)]
        let mut hydration_work: Option<SuspenseMessage> = None;

        {
            let work = dom.wait_for_work().fuse();
            pin_mut!(work);

            let mut rx_next = rx.select_next_some();
            let mut hydration_receiver_iter = futures_util::stream::iter(&mut hydration_receiver)
                .fuse()
                .flatten();
            let mut rx_hydration = hydration_receiver_iter.select_next_some();

            #[cfg(all(feature = "hot_reload", debug_assertions))]
            #[allow(unused)]
            {
                let mut hot_reload_next = hotreload_rx.select_next_some();
                select! {
                    _ = work => {
                        res = None;
                        template = None;
                    },
                    new_template = hot_reload_next => {
                        res = None;
                        template = Some(new_template);
                    },
                    evt = rx_next => {
                        res = Some(evt);
                        template = None;
                    }
                    hydration_data = rx_hydration => {
                        res = None;
                        template = None;
                        #[cfg(feature = "hydrate")]
                        {
                            hydration_work = Some(hydration_data);
                        }
                    },
                }
            }

            #[cfg(not(all(feature = "hot_reload", debug_assertions)))]
            #[allow(unused)]
            {
                select! {
                    _ = work => res = None,
                    evt = rx_next => res = Some(evt),
                    hyd = rx_hydration => {
                        res = None;
                        #[cfg(feature = "hydrate")]
                        {
                            hydration_work = Some(hyd);
                        }
                    }
                }
            }
        }

        #[cfg(all(feature = "hot_reload", debug_assertions))]
        if let Some(hr_msg) = template {
            // Replace all templates
            dioxus_hot_reload::apply_changes(&mut dom, &hr_msg);

            if !hr_msg.assets.is_empty() {
                crate::hot_reload::invalidate_browser_asset_cache();
            }
        }

        #[cfg(feature = "hydrate")]
        if let Some(hydration_data) = hydration_work {
            websys_dom.rehydrate_streaming(hydration_data, &mut dom);
        }

        // Dequeue all of the events from the channel in send order
        // todo: we should re-order these if possible
        while let Some(evt) = res {
            dom.handle_event(
                evt.name.as_str(),
                Rc::new(evt.data),
                evt.element,
                evt.bubbles,
            );
            res = rx.try_next().transpose().unwrap().ok();
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
        dom.render_immediate(&mut websys_dom);

        // wait for the animation frame to fire so we can apply our changes
        // work_loop.wait_for_raf().await;

        websys_dom.flush_edits();
    }
}
