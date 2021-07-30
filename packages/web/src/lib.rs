//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using Websys.

pub use crate::cfg::WebConfig;
use crate::dom::load_document;
use dioxus::prelude::{Context, Properties, VNode};
use dioxus::virtual_dom::VirtualDom;
pub use dioxus_core as dioxus;
use dioxus_core::error::Result;
use dioxus_core::{events::EventTrigger, prelude::FC};
use futures_util::{pin_mut, Stream, StreamExt};
use fxhash::FxHashMap;
use js_sys::Iterator;
use web_sys::{window, Document, Element, Event, Node, NodeList};

mod cache;
mod cfg;
mod dom;
mod nodeslab;

/// Launches the VirtualDOM from the specified component function.
///
/// This method will block the thread with `spawn_local`
///
/// # Example
///
///
///
pub fn launch<F>(root: FC<()>, config: F)
where
    F: FnOnce(WebConfig) -> WebConfig,
{
    launch_with_props(root, (), config)
}

/// Launches the VirtualDOM from the specified component function and props.
///
/// This method will block the thread with `spawn_local`
///
/// # Example
///
///
pub fn launch_with_props<T, F>(root: FC<T>, root_props: T, config: F)
where
    T: Properties + 'static,
    F: FnOnce(WebConfig) -> WebConfig,
{
    let config = config(WebConfig::default());
    let fut = run_with_props(root, root_props, config);

    wasm_bindgen_futures::spawn_local(async {
        match fut.await {
            Ok(_) => log::error!("Your app completed running... somehow?"),
            Err(e) => log::error!("Your app crashed! {}", e),
        }
    });
}

/// This method is the primary entrypoint for Websys Dioxus apps. Will panic if an error occurs while rendering.
/// See DioxusErrors for more information on how these errors could occour.
///
/// # Example
///
/// ```ignore
/// fn main() {
///     wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
/// }
/// ```
///
/// Run the app to completion, panicing if any error occurs while rendering.
/// Pairs well with the wasm_bindgen async handler
pub async fn run_with_props<T: Properties + 'static>(
    root: FC<T>,
    root_props: T,
    cfg: WebConfig,
) -> Result<()> {
    let mut dom = VirtualDom::new_with_props(root, root_props);

    // let tasks = dom.shared.tasks.clone();

    let root_el = load_document().get_element_by_id("dioxusroot").unwrap();
    let mut websys_dom = dom::WebsysDom::new(root_el, cfg);

    let mut edits = Vec::new();
    dom.rebuild(&mut websys_dom, &mut edits)?;
    websys_dom.process_edits(&mut edits);

    log::info!("Going into event loop");

    // #[allow(unreachable_code)]
    loop {
        let trigger = {
            let real_queue = websys_dom.wait_for_event();
            if dom.any_pending_events() {
                log::info!("tasks is not empty, waiting for either tasks or event system");
                let mut task = dom.wait_for_event();

                pin_mut!(real_queue);
                pin_mut!(task);

                match futures_util::future::select(real_queue, task).await {
                    futures_util::future::Either::Left((trigger, _)) => trigger,
                    futures_util::future::Either::Right((trigger, _)) => trigger,
                }
            } else {
                log::info!("tasks is empty, waiting for dom event to trigger soemthing");
                real_queue.await
            }
        };

        if let Some(real_trigger) = trigger {
            log::info!("event received");

            dom.queue_event(real_trigger);

            let mut edits = Vec::new();
            dom.progress_with_event(&mut websys_dom, &mut edits).await?;
            websys_dom.process_edits(&mut edits);
        }
    }

    // should actually never return from this, should be an error, rustc just cant see it
    Ok(())
}

struct HydrationNode {
    id: usize,
    node: Node,
}
