use std::collections::BTreeMap;

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
        let (depth, new_ctx) = if let Some(OutletContext {
            depth,
            mut named_depth,
        }) = cx.consume_context::<OutletContext>()
        {
            if let Some(name) = cx.props.name {
                let d = named_depth.get(name).map(|d| d + 1).unwrap_or_default();
                named_depth.insert(name.to_string(), d);
                (d, OutletContext { depth, named_depth })
            } else {
                let d = depth.map(|d| d + 1).unwrap_or_default();
                (
                    d,
                    OutletContext {
                        depth: Some(d),
                        named_depth,
                    },
                )
            }
        } else {
            if let Some(name) = cx.props.name {
                let mut named_depth = BTreeMap::new();
                named_depth.insert(name.to_string(), 0);
                (
                    0,
                    OutletContext {
                        depth: None,
                        named_depth,
                    },
                )
            } else {
                (
                    0,
                    OutletContext {
                        depth: Some(0),
                        named_depth: BTreeMap::new(),
                    },
                )
            }
        };
        cx.provide_context(new_ctx);
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
    let X = if let Some(name) = cx.props.name {
        match state.components.1.get(name) {
            Some(x) => x.get(*depth),
            None => None,
        }
    } else {
        state.components.0.get(*depth)
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
