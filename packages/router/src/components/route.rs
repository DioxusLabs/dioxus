use crate::{RouteContext, RouterContext};
use dioxus::prelude::*;

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
    let router_root = use_context::<RouterContext>(cx).unwrap();
    let root_context = use_context::<RouteContext>(cx);

    cx.use_hook(|| {
        // create a bigger, better, longer route if one above us exists
        let total_route = match root_context {
            Some(ctx) => ctx.total_route.clone(),
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

    println!("Checking Route: {:?}", cx.props.to);

    if router_root.should_render(cx.scope_id()) {
        println!("Route should render: {:?}", cx.scope_id());
        cx.render(rsx!(&cx.props.children))
    } else {
        println!("Route should *not* render: {:?}", cx.scope_id());
        cx.render(rsx!(()))
    }
}
