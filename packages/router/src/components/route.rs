use dioxus_core::Element;

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::Props;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::{RouteContext, RouterService};

#[derive(Props)]
pub struct RouteProps<'a> {
    to: &'a str,

    children: Element<'a>,

    #[props(default)]
    fallback: bool,
}

pub fn Route<'a>(cx: Scope<'a, RouteProps<'a>>) -> Element {
    // now we want to submit
    let router_root = cx
        .use_hook(|_| cx.consume_context::<RouterService>())
        .as_ref()?;

    cx.use_hook(|_| {
        // create a bigger, better, longer route if one above us exists
        let total_route = match cx.consume_context::<RouteContext>() {
            Some(ctx) => format!("{}", ctx.total_route.clone()),
            None => format!("{}", cx.props.to.clone()),
        };

        // provide our route context
        let route_context = cx.provide_context(RouteContext {
            declared_route: cx.props.to.to_string(),
            total_route,
        });

        // submit our rout
        router_root.register_total_route(
            route_context.total_route.clone(),
            cx.scope_id(),
            cx.props.fallback,
        );

        Some(RouteInner {})
    });

    log::debug!("Checking route {}", cx.props.to);

    if router_root.should_render(cx.scope_id()) {
        cx.render(rsx!(&cx.props.children))
    } else {
        None
    }
}

struct RouteInner {}

impl Drop for RouteInner {
    fn drop(&mut self) {
        // todo!()
    }
}
