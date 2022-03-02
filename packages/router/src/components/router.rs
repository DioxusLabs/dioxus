use crate::location::ParsedRoute;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::cfg::RouterCfg;
use crate::RouteEvent;
use crate::RouterCore;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core::Element;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use futures_util::stream::StreamExt;

/// The props for the [`Router`](fn.Router.html) component.
#[derive(Props)]
pub struct RouterProps<'a> {
    /// The routes and elements that should be rendered when the path matches.
    ///
    /// If elements are not contained within Routes, the will be rendered
    /// regardless of the path.
    pub children: Element<'a>,

    /// The URL to point at
    ///
    /// This will be used to trim any latent segments from the URL when your app is
    /// not deployed to the root of the domain.
    #[props(optional)]
    pub base_url: Option<&'a str>,

    /// Hook into the router when the route is changed.
    ///
    /// This lets you easily implement redirects
    #[props(default)]
    pub onchange: EventHandler<'a, Arc<RouterCore>>,
}

/// A component that conditionally renders children based on the current location of the app.
///
/// Uses BrowserRouter in the browser and HashRouter everywhere else.
///
/// Will fallback to HashRouter is BrowserRouter is not available, or through configuration.
#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    let call_onchange = cx.use_hook(|_| Rc::new(Cell::new(false)));

    let svc = cx.use_hook(|_| {
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<RouteEvent>();

        let base_url = cx.props.base_url.map(|s| s.to_string());

        let svc = RouterCore::new(tx, RouterCfg { base_url });

        cx.spawn({
            let svc = svc.clone();
            let regen_route = cx.schedule_update_any();
            let call_onchange = call_onchange.clone();
            let router_id = cx.scope_id();

            async move {
                while let Some(msg) = rx.next().await {
                    if let Some(_new) = svc.handle_route_event(msg) {
                        call_onchange.set(true);

                        regen_route(router_id);

                        for listener in svc.onchange_listeners.borrow().iter() {
                            regen_route(*listener);
                        }

                        for route in svc.slots.borrow().keys() {
                            regen_route(*route);
                        }
                    }
                }
            }
        });

        cx.provide_context(svc)
    });

    if call_onchange.get() {
        cx.props.onchange.call(svc.clone());
        call_onchange.set(false);
    }

    cx.render(rsx!(&cx.props.children))
}
