use dioxus::prelude::*;
use log::error;

use crate::{contexts::OutletContext, helpers::use_router_subscription};

/// Properties for an [`Outlet`].
#[derive(Debug, PartialEq, Props)]
pub struct OutletProps {
    /// Override the [`Outlet`]s nesting depth.
    ///
    /// By default the [`Outlet`] will find its own depth. This allows you to override that depth.
    /// Nested [`Outlet`]s will respect this override and calculate their depth based on it.
    ///
    /// Be very careful when using this prop. It is __extremely__ easy to unknowingly create an
    /// unterminated recursion with it.
    pub depth: Option<usize>,
    /// Set a side content name.
    ///
    /// By default an [`Outlet`] will only render main content. This will make it render side
    /// content defined via [`RcMulti`](crate::route_definition::RouteContent::RcMulti).
    pub name: Option<&'static str>,
}

/// Renders the content of the current route.
///
/// Only works as a descendent of a [`Router`] component, otherwise it is inactive.
///
/// The [`Outlet`] is aware of how many [`Outlet`]s it is nested within. It will render the contents
/// of the active route that is nested __exactly__ as deep.
///
/// # Panic
/// - When not nested within a [`Router`], but only in debug builds.
///
/// # Example
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     let routes = use_segment(&cx, Segment::new);
///
///     cx.render(rsx! {
///         Router {
///             routes: routes.clone(),
///             Outlet { }
///         }
///     })
/// }
/// ```
///
/// [`Router`]: crate::components::Router
#[allow(non_snake_case)]
pub fn Outlet(cx: Scope<OutletProps>) -> Element {
    let OutletProps { depth, name } = &cx.props;

    // hook up to router
    let router = match use_router_subscription(&cx) {
        Some(r) => r,
        None => {
            error!("`Outlet` can only be used as a descendent of a `Router`, will be inactive");
            #[cfg(debug_assertions)]
            panic!("`Outlet` can only be used as a descendent of a `Router`");
            #[cfg(not(debug_assertions))]
            return None;
        }
    };
    let state = router.state.read().expect("router lock poison");

    // get own depth and communicate to nested outlets
    let depth = cx.use_hook(|| {
        let mut ctx = cx.consume_context::<OutletContext>().unwrap_or_default();

        // allow depth override
        let depth = depth.unwrap_or_else(|| ctx.get_depth(*name));
        ctx.set_depth(*name, depth);

        cx.provide_context(ctx);
        depth
    });

    // get the component to render
    let (unnamed, named) = &state.components;
    let X = match cx.props.name {
        None => unnamed.get(*depth),
        Some(name) => named.get(name).and_then(|comps| comps.get(*depth)),
    }
    .copied();

    // render component or nothing
    cx.render(match X {
        Some(X) => rsx! { X {} },
        None => rsx! { Fragment {} },
    })
}
