//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using Websys.

use std::rc::Rc;

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

    let root_el = load_document().get_element_by_id(&cfg.rootname).unwrap();

    let tasks = dom.get_event_sender();

    let mut real = RealDomWebsys {};

    // initialize the virtualdom first
    if cfg.hydrate {
        dom.rebuild_in_place()?;
    }

    let mut websys_dom = dom::WebsysDom::new(
        root_el,
        cfg,
        Rc::new(move |event| tasks.unbounded_send(event).unwrap()),
    );

    dom.run(&mut websys_dom).await?;

    Ok(())
}

struct HydrationNode {
    id: usize,
    node: Node,
}

struct RealDomWebsys {}
impl dioxus::RealDom for RealDomWebsys {
    fn raw_node_as_any(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}
