use std::sync::Arc;

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::Props;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::{RouteContext, RouterCore};

/// Props for the [`Route`](struct.Route.html) component.
#[derive(Props)]
pub struct RouteProps<'a> {
    /// The path to match.
    pub to: &'a str,

    /// The component to render when the path matches.
    pub children: Element<'a>,
}

/// A component that conditionally renders children based on the current location.
///
/// # Example
///
///```rust, ignore
/// rsx!(
///     Router {
///         Route { to: "/home", Home {} }
///         Route { to: "/about", About {} }
///         Route { to: "/Blog", Blog {} }
///     }
/// )
/// ```
pub fn Route<'a>(cx: Scope<'a, RouteProps<'a>>) -> Element {
    let router_root = cx
        .use_hook(|_| cx.consume_context::<Arc<RouterCore>>())
        .as_ref()?;

    cx.use_hook(|_| {
        // create a bigger, better, longer route if one above us exists
        let total_route = match cx.consume_context::<RouteContext>() {
            Some(ctx) => ctx.total_route,
            None => cx.props.to.to_string(),
        };

        // provide our route context
        let route_context = cx.provide_context(RouteContext {
            declared_route: cx.props.to.to_string(),
            total_route,
        });

        // submit our rout
        router_root.register_total_route(route_context.total_route, cx.scope_id());
    });

    log::debug!("Checking Route: {:?}", cx.props.to);

    if router_root.should_render(cx.scope_id()) {
        log::debug!("Route should render: {:?}", cx.scope_id());
        cx.render(rsx!(&cx.props.children))
    } else {
        log::debug!("Route should *not* render: {:?}", cx.scope_id());
        None
    }
}
