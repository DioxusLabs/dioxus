use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::warn;

use crate::{
    contexts::RouterContext,
    history::HistoryProvider,
    route_definition::{RouteContent, Segment},
    service::RouterService,
};

/// The props for a [`Router`].
#[derive(Props)]
pub struct RouterProps<'a> {
    /// A class to apply to active links.
    ///
    /// Can be overwritten on individual links.
    pub active_class: Option<&'a str>,
    /// The components to render where the [`Router`] itself is. Should contain at least one
    /// [Outlet](crate::components::Outlet).
    pub children: Element<'a>,
    /// The global fallback content.
    #[props(default)]
    pub fallback: RouteContent,
    /// A function that constructs a history provider.
    pub history: Option<Box<dyn Fn() -> Box<dyn HistoryProvider>>>,
    /// If `true`, the router will perform the initial routing and then become inactive.
    #[props(default)]
    pub init_only: bool,
    /// A path that the router navigates to if a named navigation doesn't result in a path.
    pub named_navigation_fallback_path: Option<String>,
    /// The routes the router should work on.
    pub routes: &'a Segment,
}

/// The base component on which the entire router system builds.
///
/// All other router components and hooks can only be used as descendants of a [`Router`] component.
///
/// [`Router`] components cannot be nested. If you nest multiple [`Router`]s, the inner [`Router`]
/// will be inactive and ignored by all other components and hooks.
#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    cx.use_hook(|_| {
        // make sure no router context exists
        if cx.consume_context::<RouterContext>().is_some() {
            warn!("routers cannot be nested; inner router will be inactive");
            return;
        };

        // create history provider
        let history = match &cx.props.history {
            Some(x) => Some(x()),
            None => None,
        };

        // create router service and inject context
        let (mut service, context) = RouterService::new(
            cx.props.routes.clone(),
            cx.schedule_update_any(),
            cx.props.named_navigation_fallback_path.clone(),
            cx.props.active_class.map(|ac| ac.to_string()),
            cx.props.fallback.clone(),
            history,
        );
        cx.provide_context(context);

        // run service
        if cx.props.init_only {
            service.initial_routing();
        } else {
            cx.spawn(async move { service.run().await });
        }
    });

    cx.render(rsx!(&cx.props.children))
}
