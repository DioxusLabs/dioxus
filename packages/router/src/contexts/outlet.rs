use dioxus_lib::prelude::*;

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

pub(crate) fn use_outlet_context<R: 'static>() -> OutletContext<R> {
    use_hook(|| {
        try_consume_context().unwrap_or(OutletContext::<R> {
            current_level: 1,
            _marker: std::marker::PhantomData,
        })
    })
}

impl<R> OutletContext<R> {
    pub(crate) fn render() -> Element
    where
        R: Routable + Clone,
    {
        let router = use_router_internal().expect("Outlet must be inside of a router");
        let outlet: OutletContext<R> = use_outlet_context();
        let current_level = outlet.current_level;
        provide_context({
            OutletContext::<R> {
                current_level: current_level + 1,
                _marker: std::marker::PhantomData,
            }
        });

        if let Some(error) = router.render_error() {
            if current_level == 0 {
                return Some(error);
            } else {
                return None;
            }
        }

        router.current::<R>().render(current_level)
    }
}
