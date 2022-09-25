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
//! - noderefs
//! - controlled components
//! - re-hydration
//! - and more.
//!
//! The actual implementation is farily thin, with the heavy lifting happening inside the Dioxus Core crate.
//!
//! To purview the examples, check of the root Dioxus crate - the examples in this crate are mostly meant to provide
//! validation of websys-specific features and not the general use of Dioxus.

// ## RequestAnimationFrame and RequestIdleCallback
// ------------------------------------------------
// React implements "jank free rendering" by deliberately not blocking the browser's main thread. For large diffs, long
// running work, and integration with things like React-Three-Fiber, it's extremeley important to avoid blocking the
// main thread.
//
// React solves this problem by breaking up the rendering process into a "diff" phase and a "render" phase. In Dioxus,
// the diff phase is non-blocking, using "work_with_deadline" to allow the browser to process other events. When the diff phase
// is  finally complete, the VirtualDOM will return a set of "Mutations" for this crate to apply.
//
// Here, we schedule the "diff" phase during the browser's idle period, achieved by calling RequestIdleCallback and then
// setting a timeout from the that completes when the idleperiod is over. Then, we call requestAnimationFrame
//
//     From Google's guide on rAF and rIC:
//     -----------------------------------
//
//     If the callback is fired at the end of the frame, it will be scheduled to go after the current frame has been committed,
//     which means that style changes will have been applied, and, importantly, layout calculated. If we make DOM changes inside
//      of the idle callback, those layout calculations will be invalidated. If there are any kind of layout reads in the next
//      frame, e.g. getBoundingClientRect, clientWidth, etc, the browser will have to perform a Forced Synchronous Layout,
//      which is a potential performance bottleneck.
//
//     Another reason not trigger DOM changes in the idle callback is that the time impact of changing the DOM is unpredictable,
//     and as such we could easily go past the deadline the browser provided.
//
//     The best practice is to only make DOM changes inside of a requestAnimationFrame callback, since it is scheduled by the
//     browser with that type of work in mind. That means that our code will need to use a document fragment, which can then
//     be appended in the next requestAnimationFrame callback. If you are using a VDOM library, you would use requestIdleCallback
//     to make changes, but you would apply the DOM patches in the next requestAnimationFrame callback, not the idle callback.
//
//     Essentially:
//     ------------
//     - Do the VDOM work during the idlecallback
//     - Do DOM work in the next requestAnimationFrame callback

use std::rc::Rc;

pub use crate::cfg::Config;
pub use crate::util::use_eval;
use dioxus_core::prelude::Component;
use dioxus_core::SchedulerMsg;
use dioxus_core::VirtualDom;
use futures_util::FutureExt;

mod cache;
mod cfg;
mod dom;
#[cfg(feature = "hot-reload")]
mod hot_reload;
mod rehydrate;
mod ric_raf;
mod util;

/// Launch the VirtualDOM given a root component and a configuration.
///
/// This function expects the root component to not have root props. To launch the root component with root props, use
/// `launch_with_props` instead.
///
/// This method will block the thread with `spawn_local` from wasm_bindgen_futures.
///
/// If you need to run the VirtualDOM in its own thread, use `run_with_props` instead and await the future.
///
/// # Example
///
/// ```rust, ignore
/// fn main() {
///     dioxus_web::launch(App);
/// }
///
/// static App: Component = |cx| {
///     render!(div {"hello world"})
/// }
/// ```
pub fn launch(root_component: Component) {
    launch_with_props(root_component, (), Config::default());
}

/// Launch your app and run the event loop, with configuration.
///
/// This function will start your web app on the main web thread.
///
/// You can configure the WebView window with a configuration closure
///
/// ```rust, ignore
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus_web::launch_with_props(App, Config::new().pre_render(true));
/// }
///
/// fn app(cx: Scope) -> Element {
///     cx.render(rsx!{
///         h1 {"hello world!"}
///     })
/// }
/// ```
pub fn launch_cfg(root: Component, config: Config) {
    launch_with_props(root, (), config)
}

/// Launches the VirtualDOM from the specified component function and props.
///
/// This method will block the thread with `spawn_local`
///
/// # Example
///
/// ```rust, ignore
/// fn main() {
///     dioxus_web::launch_with_props(
///         App,
///         RootProps { name: String::from("joe") },
///         Config::new()
///     );
/// }
///
/// #[derive(ParitalEq, Props)]
/// struct RootProps {
///     name: String
/// }
///
/// static App: Component<RootProps> = |cx| {
///     render!(div {"hello {cx.props.name}"})
/// }
/// ```
pub fn launch_with_props<T>(root_component: Component<T>, root_properties: T, config: Config)
where
    T: Send + 'static,
{
    wasm_bindgen_futures::spawn_local(run_with_props(root_component, root_properties, config));
}

/// Runs the app as a future that can be scheduled around the main thread.
///
/// Polls futures internal to the VirtualDOM, hence the async nature of this function.
///
/// # Example
///
/// ```ignore
/// fn main() {
///     let app_fut = dioxus_web::run_with_props(App, RootProps { name: String::from("joe") });
///     wasm_bindgen_futures::spawn_local(app_fut);
/// }
/// ```
pub async fn run_with_props<T: 'static + Send>(root: Component<T>, root_props: T, cfg: Config) {
    let mut dom = VirtualDom::new_with_props(root, root_props);

    if cfg!(feature = "panic_hook") && cfg.default_panic_hook {
        console_error_panic_hook::set_once();
    }

    #[cfg(feature = "hot-reload")]
    hot_reload::init(&dom);

    for s in crate::cache::BUILTIN_INTERNED_STRINGS {
        wasm_bindgen::intern(s);
    }
    for s in &cfg.cached_strings {
        wasm_bindgen::intern(s);
    }

    let tasks = dom.get_scheduler_channel();

    let sender_callback: Rc<dyn Fn(SchedulerMsg)> =
        Rc::new(move |event| tasks.unbounded_send(event).unwrap());

    let should_hydrate = cfg.hydrate;

    let mut websys_dom = dom::WebsysDom::new(cfg, sender_callback);

    log::trace!("rebuilding app");

    if should_hydrate {
        // todo: we need to split rebuild and initialize into two phases
        // it's a waste to produce edits just to get the vdom loaded
        let _ = dom.rebuild();

        if let Err(err) = websys_dom.rehydrate(&dom) {
            log::error!(
                "Rehydration failed {:?}. Rebuild DOM into element from scratch",
                &err
            );

            websys_dom.root.set_text_content(None);

            // errrrr we should split rebuild into two phases
            // one that initializes things and one that produces edits
            let edits = dom.rebuild();

            websys_dom.apply_edits(edits.edits);
        }
    } else {
        let edits = dom.rebuild();
        websys_dom.apply_edits(edits.edits);
    }

    let mut work_loop = ric_raf::RafLoop::new();

    loop {
        log::trace!("waiting for work");
        // if virtualdom has nothing, wait for it to have something before requesting idle time
        // if there is work then this future resolves immediately.
        dom.wait_for_work().await;

        log::trace!("working..");

        // wait for the mainthread to schedule us in
        let mut deadline = work_loop.wait_for_idle_time().await;

        // run the virtualdom work phase until the frame deadline is reached
        let mutations = dom.work_with_deadline(|| (&mut deadline).now_or_never().is_some());

        // wait for the animation frame to fire so we can apply our changes
        work_loop.wait_for_raf().await;

        for edit in mutations {
            // actually apply our changes during the animation frame
            websys_dom.apply_edits(edit.edits);
        }
    }
}
