use dioxus::prelude::*;

use crate::{routable::Routable, utils::use_router_internal::use_router_internal};

#[derive(Clone)]
pub(crate) struct OutletContext {
    pub current_level: usize,
}

pub(crate) fn use_outlet_context(cx: &ScopeState) -> &OutletContext {
    let outlet_context = cx.use_hook(|| {
        cx.consume_context()
            .unwrap_or(OutletContext { current_level: 0 })
    });
    outlet_context
}

impl OutletContext {
    pub(crate) fn render<R: Routable + Clone>(cx: Scope) -> Element<'_> {
        let router = use_router_internal::<R>(cx)
            .as_ref()
            .expect("Outlet must be inside of a router");
        let outlet = use_outlet_context(cx);
        let current_level = outlet.current_level;
        cx.provide_context({
            OutletContext {
                current_level: current_level + 1,
            }
        });

        if let Some(error) = router.render_error(cx) {
            if current_level == 0 {
                return Some(error);
            } else {
                return None;
            }
        }

        router.current().render(cx, current_level)
    }
}
