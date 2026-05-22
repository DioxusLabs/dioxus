#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

//! # Dioxus Web

pub use crate::cfg::Config;
use crate::hydration::SuspenseMessage;
use dioxus_core::{
    RenderCheckpoint, RenderCommit, RenderSchedulerDecision, UpdatePriority, VirtualDom,
};
use dom::WebsysDom;
use futures_util::{FutureExt, StreamExt, pin_mut, select};
use std::{cell::Cell, cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

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

const HOST_YIELD_INTERVAL_MS: f64 = 5.0;

#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function publishSchedulerStats(workCount, commitCount, yieldCount) {
    if (typeof window === "undefined") {
        return;
    }

    const detail = {
        workCount,
        commitCount,
        yieldCount,
        timestamp: performance.now(),
    };

    window.__dioxusFiberStats = detail;
    window.dispatchEvent(new CustomEvent("dioxus-fiber-stats", { detail }));
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = publishSchedulerStats)]
    fn publish_scheduler_stats(work_count: u32, commit_count: u32, yield_count: u32);
}

fn publish_commit_stats(commit: RenderCommit) {
    publish_scheduler_stats(commit.work_count as u32, 1, 0);
}

#[derive(Clone)]
struct BrowserHostScheduler {
    next_yield: Rc<Cell<f64>>,
    pending_input: Rc<Cell<bool>>,
}

impl BrowserHostScheduler {
    fn new() -> Self {
        Self {
            next_yield: Rc::new(Cell::new(performance_now() + HOST_YIELD_INTERVAL_MS)),
            pending_input: Rc::new(Cell::new(false)),
        }
    }

    fn checkpoint(&self, checkpoint: RenderCheckpoint) -> RenderSchedulerDecision {
        let input_pending = is_input_pending();
        self.pending_input.set(input_pending);

        if input_pending {
            return RenderSchedulerDecision::CommitAndYield;
        }

        if checkpoint.has_higher_priority_work {
            return RenderSchedulerDecision::CommitAndYield;
        }

        if checkpoint.priority <= UpdatePriority::ContinuousInput
            && checkpoint.pending_mutations > 0
        {
            return RenderSchedulerDecision::Commit;
        }

        if performance_now() >= self.next_yield.get() {
            RenderSchedulerDecision::CommitAndYield
        } else {
            RenderSchedulerDecision::Continue
        }
    }

    async fn yield_to_host(&self) {
        self.yield_to_host_after_priority(UpdatePriority::Default)
            .await;
    }

    async fn yield_to_host_after_priority(&self, priority: UpdatePriority) {
        if self.pending_input.replace(false) || priority == UpdatePriority::SyncInput {
            request_animation_frame().await;
        }
        gloo_timers::future::TimeoutFuture::new(0).await;
        self.reset_yield_deadline();
    }

    fn reset_yield_deadline(&self) {
        self.next_yield
            .set(performance_now() + HOST_YIELD_INTERVAL_MS);
    }
}

fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or_default()
}

fn is_input_pending() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };

    let navigator = window.navigator();
    let Ok(scheduling) = js_sys::Reflect::get(navigator.as_ref(), &JsValue::from_str("scheduling"))
    else {
        return false;
    };
    if scheduling.is_null() || scheduling.is_undefined() {
        return false;
    }

    let Ok(is_input_pending) =
        js_sys::Reflect::get(&scheduling, &JsValue::from_str("isInputPending"))
    else {
        return false;
    };
    let Some(is_input_pending) = is_input_pending.dyn_ref::<js_sys::Function>() else {
        return false;
    };

    is_input_pending
        .call0(&scheduling)
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

async fn request_animation_frame() {
    let Some(window) = web_sys::window() else {
        gloo_timers::future::TimeoutFuture::new(0).await;
        return;
    };

    let (sender, receiver) = futures_channel::oneshot::channel();
    let sender = Rc::new(RefCell::new(Some(sender)));
    let callback_slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));

    let sender_for_callback = sender.clone();
    let callback_slot_for_callback = callback_slot.clone();
    let callback = Closure::wrap(Box::new(move |_| {
        if let Some(sender) = sender_for_callback.borrow_mut().take() {
            _ = sender.send(());
        }
        callback_slot_for_callback.borrow_mut().take();
    }) as Box<dyn FnMut(f64)>);

    if window
        .request_animation_frame(callback.as_ref().unchecked_ref())
        .is_err()
    {
        gloo_timers::future::TimeoutFuture::new(0).await;
        return;
    }

    *callback_slot.borrow_mut() = Some(callback);
    _ = receiver.await;
}

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
        virtual_dom.in_scope(dioxus_core::ScopeId::ROOT, || {
            dioxus_core::provide_context(history)
        });
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
                    if let Some(msg) = msg
                        && msg.for_build_id == Some(dioxus_cli_config::build_id()) {
                            dioxus_devtools::apply_changes(&virtual_dom, &msg);
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
                virtual_dom.in_runtime(|| virtual_dom.runtime().throw_error(dioxus_core::ScopeId::APP, error));
            }
            server_data.in_context(|| {
                virtual_dom.in_scope(dioxus_core::ScopeId::ROOT, || {
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

        let scheduler = BrowserHostScheduler::new();
        virtual_dom
            .render_concurrent_with_scheduler(
                &mut websys_dom,
                |checkpoint, _| scheduler.checkpoint(checkpoint),
                |websys_dom, commit| {
                    publish_commit_stats(commit);
                    websys_dom.flush_edits();
                    scheduler.reset_yield_deadline();
                },
                |priority| {
                    let scheduler = scheduler.clone();
                    async move {
                        publish_scheduler_stats(0, 0, 1);
                        match priority {
                            Some(priority) => {
                                scheduler.yield_to_host_after_priority(priority).await
                            }
                            None => scheduler.yield_to_host().await,
                        }
                    }
                },
            )
            .await;
    }
}
