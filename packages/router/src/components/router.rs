use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::warn;

use crate::{contexts::RouterContext, route_definition::Segment, service::RouterService};

/// The props for a [`Router`].
#[derive(Props)]
pub struct RouterProps<'a> {
    /// The components to render where the [`Router`] itself is. Should contain at least one
    /// [Outlet](crate::components::Outlet).
    pub children: Element<'a>,
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

        // create router service and inject context
        let (mut service, context) = RouterService::new(
            cx.props.routes.clone(),
            cx.schedule_update_any(),
            cx.props.named_navigation_fallback_path.clone(),
        );
        cx.provide_context(context);

        // run service
        cx.spawn(async move { service.run().await });
    });

    cx.render(rsx!(&cx.props.children))
}
