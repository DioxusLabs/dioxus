use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{contexts::OutletContext, helpers::sub_to_router};

/// Properties for an [`Outlet`].
#[derive(PartialEq, Props)]
pub struct OutletProps {
    /// The name of the side_content to render. Will render main content if absent.
    pub name: Option<&'static str>,
}

/// An outlet tells the router where to render the components corresponding to the current route.
///
/// Needs a [Router](crate::components::Router) as an ancestor.
///
/// Each [`Outlet`] renders a single component. To render the components of nested routes simply
/// provide nested [`Outlet`]s.
#[allow(non_snake_case)]
pub fn Outlet(cx: Scope<OutletProps>) -> Element {
    // get own depth and communicate to lower outlets
    let depth = cx.use_hook(|_| {
        let higher = cx.consume_context::<OutletContext>();
        let depth = higher.map(|ctx| ctx.depth + 1).unwrap_or(0);
        cx.provide_context(OutletContext { depth });
        depth
    });

    // get router state and register for updates
    let router = match sub_to_router(&cx) {
        Some(r) => r,
        None => {
            error!("`Outlet` can only be used as a descendent of a `Router`");
            return None;
        }
    };

    // get the component to render
    let state = router.state.read().unwrap();
    let X = match state.components.get(*depth) {
        Some((main, side)) => match cx.props.name {
            Some(name) => side.get(name),
            None => Some(main),
        },
        None => None,
    };

    // render component or nothing
    match X {
        Some(X) => {
            let X = *X;
            cx.render(rsx! { X {} })
        }
        None => cx.render(rsx! { Fragment { } }),
    }
}
