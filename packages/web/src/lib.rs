#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

//! # Dioxus Web

pub use crate::cfg::Config;
use crate::hydration::SuspenseMessage;
use dioxus_core::{ScopeId, VirtualDom};
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
#[cfg(feature = "document")]
mod history;
#[cfg(feature = "document")]
pub use document::WebDocument;
#[cfg(feature = "document")]
pub use history::{HashHistory, WebHistory};

mod files;
pub use files::*;

mod data_transfer;
pub use data_transfer::*;

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
    #[cfg(all(feature = "devtools", debug_assertions))]
    let mut hotreload_rx = devtools::init(&web_config);

    #[cfg(feature = "document")]
    if let Some(history) = web_config.history.clone() {
        virtual_dom.in_scope(ScopeId::ROOT, || dioxus_core::provide_context(history));
    }

    #[cfg(feature = "document")]
    virtual_dom.in_runtime(document::init_document);

    let runtime = virtual_dom.runtime();

    // If the hydrate feature is enabled, launch the client with hydration enabled
    let should_hydrate = web_config.hydrate || cfg!(feature = "hydrate");

    let mut websys_dom = WebsysDom::new(web_config, runtime);

    let mut hydration_receiver: Option<futures_channel::mpsc::UnboundedReceiver<SuspenseMessage>> =
        None;

    if should_hydrate {
        // If we are hydrating, then the hotreload message might actually have a patch for us to apply.
        // Let's wait for a moment to see if we get a hotreload message before we start hydrating.
        // That way, the hydration will use the same functions that the server used to serialize the data.
        #[cfg(all(feature = "devtools", debug_assertions))]
        loop {
            let mut timeout = gloo_timers::future::TimeoutFuture::new(100).fuse();
            futures_util::select! {
                msg = hotreload_rx.next() => {
                    if let Some(msg) = msg {
                        if msg.for_build_id == Some(dioxus_cli_config::build_id()) {
                            dioxus_devtools::apply_changes(&virtual_dom, &msg);
                        }
                    }
                }
                _ = &mut timeout => {
                    break;
                }
            }
        }

        #[cfg(feature = "hydrate")]
        {
            use dioxus_fullstack_core::HydrationContext;

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
                HydrationContext::from_serialized(&hydration_data, debug_types, debug_locations);
            // If the server serialized an error into the root suspense boundary, throw it into the root scope
            if let Some(error) = server_data.error_entry().get().ok().flatten() {
                virtual_dom.in_runtime(|| virtual_dom.runtime().throw_error(ScopeId::APP, error));
            }
            server_data.in_context(|| {
                virtual_dom.in_scope(ScopeId::ROOT, || {
                    // Provide a hydration compatible create error boundary method
                    dioxus_core::provide_create_error_boundary(
                        dioxus_fullstack_core::init_error_boundary,
                    );
                    #[cfg(feature = "document")]
                    document::init_fullstack_document();
                });
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

            if hr_msg.for_build_id == Some(dioxus_cli_config::build_id()) {
                devtools::show_toast(
                    "Hot-patch success!",
                    &format!("App successfully patched in {} ms", hr_msg.ms_elapsed),
                    devtools::ToastLevel::Success,
                    std::time::Duration::from_millis(2000),
                    false,
                );
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
