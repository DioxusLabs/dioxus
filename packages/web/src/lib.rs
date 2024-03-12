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

use std::rc::Rc;

pub use crate::cfg::Config;
#[cfg(feature = "file_engine")]
pub use crate::file_engine::WebFileEngineExt;
use dioxus_core::VirtualDom;
use futures_util::{pin_mut, select, FutureExt, StreamExt};

mod cfg;
mod dom;

mod event;
pub mod launch;
mod mutations;
pub use event::*;

#[cfg(feature = "eval")]
mod eval;

#[cfg(feature = "file_engine")]
mod file_engine;

#[cfg(all(feature = "hot_reload", debug_assertions))]
mod hot_reload;

#[cfg(feature = "hydrate")]
mod rehydrate;

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
pub async fn run(virtual_dom: VirtualDom, web_config: Config) {
    tracing::info!("Starting up");

    let mut dom = virtual_dom;

    #[cfg(feature = "eval")]
    dom.in_runtime(eval::init_eval);

    #[cfg(feature = "panic_hook")]
    if web_config.default_panic_hook {
        console_error_panic_hook::set_once();
    }

    #[cfg(all(feature = "hot_reload", debug_assertions))]
    let mut hotreload_rx = hot_reload::init();

    let (tx, mut rx) = futures_channel::mpsc::unbounded();

    let should_hydrate = web_config.hydrate;

    let mut websys_dom = dom::WebsysDom::new(web_config, tx);

    tracing::info!("rebuilding app");

    if should_hydrate {
        #[cfg(feature = "hydrate")]
        {
            dom.rebuild(&mut crate::rehydrate::OnlyWriteTemplates(&mut websys_dom));

            if let Err(err) = websys_dom.rehydrate(&dom) {
                tracing::error!("Rehydration failed. {:?}", err);
                tracing::error!("Rebuild DOM into element from scratch");
                websys_dom.root.set_text_content(None);

                dom.rebuild(&mut websys_dom);

                websys_dom.flush_edits();
            }
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
        let (mut res, template) = {
            let work = dom.wait_for_work().fuse();
            pin_mut!(work);

            let mut rx_next = rx.select_next_some();

            #[cfg(all(feature = "hot_reload", debug_assertions))]
            {
                let mut hot_reload_next = hotreload_rx.select_next_some();
                select! {
                    _ = work => (None, None),
                    new_template = hot_reload_next => (None, Some(new_template)),
                    evt = rx_next => (Some(evt), None),
                }
            }

            #[cfg(not(all(feature = "hot_reload", debug_assertions)))]
            select! {
                _ = work => (None, None),
                evt = rx_next => (Some(evt), None),
            }
        };

        if let Some(template) = template {
            dom.replace_template(template);
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
        // 3. Stop diffing if the deadline is exceded
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
