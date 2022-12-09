use dioxus::prelude::*;
use dioxus_router_core::{Name, OutletData};
use log::error;

use crate::utils::use_router_internal::use_router_internal;

#[derive(Debug, Eq, PartialEq, Props)]
pub struct OutletProps {
    pub depth: Option<usize>,
    pub name: Option<Name>,
}

#[allow(non_snake_case)]
pub fn Outlet(cx: Scope<OutletProps>) -> Element {
    let OutletProps { depth, name } = cx.props;

    // hook up to router
    let router = match use_router_internal(&cx) {
        Some(r) => r,
        None => {
            let msg = "`Outlet` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            #[cfg(not(debug_assertions))]
            anyhow::bail!("{msg}");
        }
    };
    let state = loop {
        if let Some(state) = router.state.try_read() {
            break state;
        }
    };

    // do depth calculation and propagation
    let depth = cx.use_hook(|| {
        let mut context = cx.consume_context::<OutletData>().unwrap_or_default();
        let depth = depth
            .or_else(|| context.depth(name))
            .map(|d| d + 1)
            .unwrap_or_default();
        context.set_depth(name, depth);
        cx.provide_context(context);
        depth
    });

    // get content
    let content = match name {
        None => state.content.get(*depth),
        Some(n) => state.named_content.get(n).and_then(|n| n.get(*depth)),
    }
    .cloned();

    cx.render(match content {
        Some(content) => {
            let X = content.0;
            rsx! { X { } }
        }
        None => rsx! { div { } },
    })
}
