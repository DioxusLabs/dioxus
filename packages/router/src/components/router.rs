use dioxus_lib::prelude::*;

use crate::{
    prelude::{Outlet, RouterContext},
    routable::Routable,
    router_cfg::RouterConfig,
};

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Routable> {
    #[props(default, into)]
    config: Callback<(), RouterConfig<R>>,
}

impl<T: Routable> Clone for RouterProps<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Routable> Copy for RouterProps<T> {}
impl<R: Routable> Default for RouterProps<R> {
    fn default() -> Self {
        Self {
            config: Callback::new(|_| RouterConfig::default()),
        }
    }
}

impl<R: Routable> PartialEq for RouterProps<R> {
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

/// A component that renders the current route.
pub fn Router<R: Routable>(props: RouterProps<R>) -> Element {
    use crate::prelude::{outlet::OutletContext, RouterContext};

    use_hook(|| {
        let ctx = RouterContext::new(props.config.call(()));
        if root_router().is_none() {
            ScopeId::ROOT.provide_context(RootRouterContext(Signal::new_in_scope(
                Some(ctx),
                ScopeId::ROOT,
            )));
        }
        provide_context(ctx);

        provide_context(OutletContext::<R> {
            current_level: 0,
            _marker: std::marker::PhantomData,
        });
    });

    rsx! {
        Outlet::<R> {}
    }
}

/// This context is set in the root of the virtual dom if there is a router present.
#[derive(Clone, Copy)]
pub(crate) struct RootRouterContext(pub(crate) Signal<Option<RouterContext>>);

/// Try to get the router that was created closest to the root of the virtual dom. This may be called outside of the router.
///
/// This will return `None` if there is no router present or the router has not been created yet.
pub fn root_router() -> Option<RouterContext> {
    if let Some(ctx) = ScopeId::ROOT.consume_context::<RootRouterContext>() {
        ctx.0.cloned()
    } else {
        ScopeId::ROOT.provide_context(RootRouterContext(Signal::new_in_scope(None, ScopeId::ROOT)));
        None
    }
}
