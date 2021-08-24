//! Dioxus WebSys
//!
//! ## Overview
//! ------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using WebSys. This web render for
//! Dioxus is one of the more advanced renderers, supporting:
//! - idle work
//! - animations
//! - jank-free rendering
//! - noderefs
//! - controlled components
//! - re-hydration
//! - and more.
//!
//! The actual implementation is farily thin, with the heavy lifting happening inside the Dioxus Core crate.
//!
//! To purview the examples, check of the root Dioxus crate - the examples in this crate are mostly meant to provide
//! validation of websys-specific features and not the general use of Dioxus.
//!
//! ## RequestAnimationFrame and RequestIdleCallback
//! ------------------------------------------------
//! React implements "jank free rendering" by deliberately not blocking the browser's main thread. For large diffs, long
//! running work, and integration with things like React-Three-Fiber, it's extremeley important to avoid blocking the
//! main thread.
//!
//! React solves this problem by breaking up the rendering process into a "diff" phase and a "render" phase. In Dioxus,
//! the diff phase is non-blocking, using "yield_now" to allow the browser to process other events. When the diff phase
//! is  finally complete, the VirtualDOM will return a set of "Mutations" for this crate to apply.
//!
//! Here, we schedule the "diff" phase during the browser's idle period, achieved by calling RequestIdleCallback and then
//! setting a timeout from the that completes when the idleperiod is over. Then, we call requestAnimationFrame
//!
//!     From Google's guide on rAF and rIC:
//!     -----------------------------------
//!
//!     If the callback is fired at the end of the frame, it will be scheduled to go after the current frame has been committed,
//!     which means that style changes will have been applied, and, importantly, layout calculated. If we make DOM changes inside
//!      of the idle callback, those layout calculations will be invalidated. If there are any kind of layout reads in the next
//!      frame, e.g. getBoundingClientRect, clientWidth, etc, the browser will have to perform a Forced Synchronous Layout,
//!      which is a potential performance bottleneck.
//!
//!     Another reason not trigger DOM changes in the idle callback is that the time impact of changing the DOM is unpredictable,
//!     and as such we could easily go past the deadline the browser provided.
//!
//!     The best practice is to only make DOM changes inside of a requestAnimationFrame callback, since it is scheduled by the
//!     browser with that type of work in mind. That means that our code will need to use a document fragment, which can then
//!     be appended in the next requestAnimationFrame callback. If you are using a VDOM library, you would use requestIdleCallback
//!     to make changes, but you would apply the DOM patches in the next requestAnimationFrame callback, not the idle callback.
//!
//!     Essentially:
//!     ------------
//!     - Do the VDOM work during the idlecallback
//!     - Do DOM work in the next requestAnimationFrame callback

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

    let hydrating = cfg.hydrate;

    let root_el = load_document().get_element_by_id(&cfg.rootname).unwrap();
    let tasks = dom.get_event_sender();
    let sender_callback = Rc::new(move |event| tasks.unbounded_send(event).unwrap());

    let mut websys_dom = dom::WebsysDom::new(root_el, cfg, sender_callback);

    let mut mutations = dom.rebuild().unwrap();
    log::info!("Mutations: {:#?}", mutations);

    // hydrating is simply running the dom for a single render. If the page is already written, then the corresponding
    // ElementIds should already line up because the web_sys dom has already loaded elements with the DioxusID into memory
    if !hydrating {
        websys_dom.process_edits(&mut mutations.edits);
    }

    loop {
        let deadline = gloo_timers::future::TimeoutFuture::new(16);
        let mut mutations = dom.run_with_deadline(deadline).await?;

        websys_dom.process_edits(&mut mutations.edits);
    }

    Ok(())
}
