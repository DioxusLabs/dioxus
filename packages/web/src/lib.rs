//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using Websys.

use dioxus::prelude::{Context, Properties, VNode};
use dioxus::virtual_dom::VirtualDom;
pub use dioxus_core as dioxus;
use dioxus_core::{events::EventTrigger, prelude::FC};
use futures_util::{pin_mut, Stream, StreamExt};
use fxhash::FxHashMap;
use web_sys::{window, Document, Element, Event, Node};

mod cache;
mod new;

/// Launches the VirtualDOM from the specified component function.
///
/// This method will block the thread with `spawn_local`
pub fn launch<F>(root: FC<()>, config: F)
where
    F: FnOnce(()),
{
    wasm_bindgen_futures::spawn_local(run(root))
}

pub fn launch_with_props<T, F>(root: FC<T>, root_props: T, config: F)
where
    T: Properties + 'static,
    F: FnOnce(()),
{
    wasm_bindgen_futures::spawn_local(run_with_props(root, root_props))
}

/// This method is the primary entrypoint for Websys Dioxus apps. Will panic if an error occurs while rendering.
/// See DioxusErrors for more information on how these errors could occour.
///
/// ```ignore
/// fn main() {
///     wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
/// }
/// ```
///
/// Run the app to completion, panicing if any error occurs while rendering.
/// Pairs well with the wasm_bindgen async handler
pub async fn run(root: FC<()>) {
    run_with_props(root, ()).await;
}

pub async fn run_with_props<T: Properties + 'static>(root: FC<T>, root_props: T) {
    let dom = VirtualDom::new_with_props(root, root_props);
    event_loop(dom).await.expect("Event loop failed");
}

pub async fn event_loop(mut internal_dom: VirtualDom) -> dioxus_core::error::Result<()> {
    use wasm_bindgen::JsCast;

    let root = prepare_websys_dom();
    let root_node = root.clone().dyn_into::<Node>().unwrap();

    let mut websys_dom = crate::new::WebsysDom::new(root.clone());

    websys_dom.stack.push(root_node.clone());
    websys_dom.stack.push(root_node);

    let mut edits = Vec::new();
    internal_dom.rebuild(&mut websys_dom, &mut edits)?;
    websys_dom.process_edits(&mut edits);

    log::info!("Going into event loop");
    loop {
        let trigger = {
            let real_queue = websys_dom.wait_for_event();
            if internal_dom.tasks.is_empty() {
                log::info!("tasks is empty, waiting for dom event to trigger soemthing");
                real_queue.await
            } else {
                log::info!("tasks is not empty, waiting for either tasks or event system");
                let task_queue = (&mut internal_dom.tasks).next();

                pin_mut!(real_queue);
                pin_mut!(task_queue);

                match futures_util::future::select(real_queue, task_queue).await {
                    futures_util::future::Either::Left((trigger, _)) => trigger,
                    futures_util::future::Either::Right((trigger, _)) => trigger,
                }
            }
        };

        if let Some(real_trigger) = trigger {
            log::info!("event received");

            internal_dom.queue_event(real_trigger)?;

            let mut edits = Vec::new();
            internal_dom
                .progress_with_event(&mut websys_dom, &mut edits)
                .await?;
            websys_dom.process_edits(&mut edits);
        }
    }

    // should actually never return from this, should be an error, rustc just cant see it
    Ok(())
}

fn prepare_websys_dom() -> Element {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
        .get_element_by_id("dioxusroot")
        .unwrap()
}
