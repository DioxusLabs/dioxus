use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{contexts::OutletContext, helpers::sub_to_router};

/// Properties for an [`Outlet`].
#[derive(PartialEq, Props)]
pub struct OutletProps {
    /// Override the [`Outlet`]s depth.
    ///
    /// By default an outlets depth will increase with each nesting. This allows you to override
    /// that depth. Descendants will ignore this override.
    ///
    /// Be careful when using this option. It is very easy to create a recursive component with it.
    pub depth: Option<usize>,
    /// The name of the side_content to render. Will render main content if absent.
    pub name: Option<&'static str>,
}

/// An outlet tells the router where to render the components corresponding to the current route.
///
/// Needs a [`Router`] as an ancestor.
///
/// Each [`Outlet`] renders a single component. To render the components of nested routes simply
/// provide nested [`Outlet`]s.
///
/// # Panic
/// When no [`Router`] is an ancestor, but only in debug builds.
///
/// [`Router`]: crate::components::Router
#[allow(non_snake_case)]
pub fn Outlet(cx: Scope<OutletProps>) -> Element {
    // hook up to router
    let router = match sub_to_router(&cx) {
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
    let depth = cx.use_hook(|_| {
        let (depth, new_ctx) = match cx.consume_context::<OutletContext>() {
            Some(mut ctx) => {
                let depth = ctx.get_depth(cx.props.name);
                ctx.set_depth(cx.props.name, depth);
                (depth, ctx)
            }
            None => (0, OutletContext::new(cx.props.name)),
        };
        cx.provide_context(new_ctx);
        depth
    });

    // allow depth override
    let depth = cx.props.depth.unwrap_or(*depth);

    // get the component to render
    let (unnamed, named) = &state.components;
    let X = match cx.props.name {
        None => unnamed.get(depth),
        Some(name) => named.get(name).and_then(|comps| comps.get(depth)),
    }
    .copied();

    // render component or nothing
    cx.render(match X {
        Some(X) => rsx! { X {} },
        None => rsx! { Fragment {} },
    })
}
