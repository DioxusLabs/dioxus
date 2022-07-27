use dioxus::prelude::*;
use std::sync::Arc;

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
    // Initialize route with first render true. It will be changed to false on the first render.
    // This is done to make sure we get every sibling route registered before we choose one to render.
    // If we don't do this, we need to have routes in a specific order to avoid flickering.
    //
    // For example, the following woud flicker when navigating to /blog/1, rendering /blog first,
    // then registering /blog/:id, and re rendering /blog to hide it.
    //
    //```rust, ignore
    // rsx!(
    //     Router {
    //         Route { to: "/blog", Blog {} }
    //         Route { to: "/blog/:id", BlogItem {} }
    //     }
    // )
    //```
    //
    // Flickering would not happen if we have the more specific route registered first.
    // For example, the following would not flicker even if we remove this first render restriction:
    //
    //```rust, ignore
    // rsx!(
    //     Router {
    //         Route { to: "/blog/:id", BlogItem {} }
    //         Route { to: "/blog", Blog {} }
    //     }
    // )
    //```
    let first_render = use_state(&cx, || true);

    let router_root = cx
        .use_hook(|| cx.consume_context::<Arc<RouterCore>>())
        .as_ref()?;

    let parent_context = cx.use_hook(|| cx.consume_context::<RouteContext>());

    let parent_scope_id = match parent_context {
        Some(context) => Some(context.scope_id),
        None => None,
    };

    cx.use_hook(|| {
        // create a bigger, better, longer route if one above us exists
        let total_route = match parent_context {
            Some(ctx) => {
                // concat parent route, making sure we have a single slash in between
                format!(
                    "{}/{}",
                    ctx.total_route.trim_end_matches("/"),
                    cx.props.to.trim_start_matches("/")
                )
            }
            None => cx.props.to.to_string(),
        };

        // provide our route context
        let route_context = cx.provide_context(RouteContext {
            declared_route: cx.props.to.to_string(),
            total_route,
            scope_id: cx.scope_id(),
        });

        // submit our rout
        router_root.register_total_route(route_context.total_route, parent_scope_id, cx.scope_id());
    });

    let _ = cx.use_hook(|| RouteUnmountListener {
        scope_id: cx.scope_id(),
        router: router_root.clone(),
        parent_scope_id: parent_scope_id,
    });

    log::trace!("Checking Route: {:?}", cx.props.to);

    if *first_render.get() {
        first_render.set(false);
        log::trace!("First render of Route: {:?}", cx.scope_id());
        None
    } else if router_root.should_render(cx.scope_id(), parent_scope_id) {
        log::trace!("Route should render: {:?}", cx.scope_id());
        cx.render(rsx!(&cx.props.children))
    } else {
        log::trace!("Route should *not* render: {:?}", cx.scope_id());
        None
    }
}

// This struct is used to know when the component is unmounted,
// so we can remove it from the router on drop.
struct RouteUnmountListener {
    scope_id: ScopeId,
    parent_scope_id: Option<ScopeId>,
    router: Arc<RouterCore>,
}

impl Drop for RouteUnmountListener {
    fn drop(&mut self) {
        self.router
            .unregister_total_route(self.scope_id, self.parent_scope_id);
    }
}
