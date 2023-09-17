use dioxus::prelude::*;

use crate::{routable::Routable, utils::use_router_internal::use_router_internal};

pub(crate) struct OutletContext<R> {
    pub current_level: usize,
    pub _marker: std::marker::PhantomData<R>,
}

impl<R> Clone for OutletContext<R> {
    fn clone(&self) -> Self {
        OutletContext {
            current_level: self.current_level,
            _marker: std::marker::PhantomData,
        }
    }
}

pub(crate) fn use_outlet_context<R: 'static>(cx: &ScopeState) -> &OutletContext<R> {
    let outlet_context = cx.use_hook(|| {
        cx.consume_context().unwrap_or(OutletContext::<R> {
            current_level: 1,
            _marker: std::marker::PhantomData,
        })
    });
    outlet_context
}

impl<R> OutletContext<R> {
    pub(crate) fn render(cx: Scope) -> Element<'_>
    where
        R: Routable + Clone,
    {
        let router = use_router_internal(cx)
            .as_ref()
            .expect("Outlet must be inside of a router");
        let outlet: &OutletContext<R> = use_outlet_context(cx);
        let current_level = outlet.current_level;
        cx.provide_context({
            OutletContext::<R> {
                current_level: current_level + 1,
                _marker: std::marker::PhantomData,
            }
        });

        if let Some(error) = router.render_error(cx) {
            if current_level == 0 {
                return Some(error);
            } else {
                return None;
            }
        }

        router.current::<R>().render(cx, current_level)
    }
}
