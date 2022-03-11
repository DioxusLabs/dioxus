use crate::ParsedRoute;
use crate::{cfg::RouterCfg, RouteEvent, RouterCore};
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use futures_util::stream::StreamExt;
use std::sync::Arc;

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
    let svc = cx.use_hook(|_| {
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<RouteEvent>();

        let base_url = cx.props.base_url.map(|s| s.to_string());

        let svc = RouterCore::new(tx, RouterCfg { base_url });

        cx.spawn({
            let svc = svc.clone();
            let regen_route = cx.schedule_update_any();
            let router_id = cx.scope_id();

            async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        RouteEvent::Push {
                            route,
                            serialized_state,
                            title,
                        } => {
                            let new_route = Arc::new(ParsedRoute {
                                url: svc.current_location().url.join(&route).ok().unwrap(),
                                title,
                                serialized_state,
                            });

                            svc.history.push(&new_route);
                            svc.stack.borrow_mut().push(new_route);
                        }

                        RouteEvent::Replace {
                            route,
                            title,
                            serialized_state,
                        } => {
                            let new_route = Arc::new(ParsedRoute {
                                url: svc.current_location().url.join(&route).ok().unwrap(),
                                title,
                                serialized_state,
                            });

                            svc.history.replace(&new_route);
                            *svc.stack.borrow_mut().last_mut().unwrap() = new_route;
                        }

                        RouteEvent::Pop => {
                            let mut stack = svc.stack.borrow_mut();

                            if stack.len() == 1 {
                                continue;
                            }

                            stack.pop();
                        }
                    }

                    svc.route_found.set(None);

                    regen_route(router_id);

                    for listener in svc.onchange_listeners.borrow().iter() {
                        regen_route(*listener);
                    }

                    for route in svc.ordering.borrow().iter().rev() {
                        regen_route(*route);
                    }
                }
            }
        });

        cx.provide_context(svc)
    });

    // next time we run the rout_found will be filled
    if svc.route_found.get().is_none() {
        cx.props.onchange.call(svc.clone());
    }

    cx.render(rsx!(&cx.props.children))
}
