use std::str::FromStr;

use crate::{
    provide_router_context, routable::Routable, router_cfg::RouterConfig, Outlet, SiteMapSegment,
};
use dioxus_core::{provide_context, use_hook, Callback, Element, VNode};
use dioxus_core_macro::{component, rsx, Props};
use dioxus_signals::{GlobalSignal, Owner, ReadableExt};

/// The props for [`Router`].
#[derive(Props)]
pub struct RouterProps<R: Clone + 'static = EmptyRoutable> {
    #[props(default, into)]
    config: Callback<(), RouterConfig<R>>,

    children: Element,
}

impl<T: Clone> Clone for RouterProps<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            children: self.children.clone(),
        }
    }
}

/// A routable type that represents an empty route.
#[derive(Clone, PartialEq)]
pub struct EmptyRoutable;

impl std::fmt::Display for EmptyRoutable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmptyRoutable")
    }
}

impl FromStr for EmptyRoutable {
    type Err = String;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(EmptyRoutable)
    }
}

impl Routable for EmptyRoutable {
    #[doc = " Render the route at the given level"]
    fn render(&self, level: usize) -> Element {
        todo!()
    }

    #[doc = " The error that can occur when parsing a route."]
    const SITE_MAP: &'static [crate::SiteMapSegment] = &[];
}

// impl Routable for () {
//     #[doc = " The error that can occur when parsing a route."]
//     const SITE_MAP: &'static [SiteMapSegment] = &[];

//     #[doc = " Render the route at the given level"]
//     fn render(&self, level: usize) -> Element {
//         todo!()
//     }
// }

// impl<T: Clone> Copy for RouterProps<T> {}

impl<R: Clone + 'static> Default for RouterProps<R> {
    fn default() -> Self {
        Self {
            config: Callback::new(|_| RouterConfig::default()),
            children: VNode::empty(),
        }
    }
}

impl<R: Clone> PartialEq for RouterProps<R> {
    fn eq(&self, _: &Self) -> bool {
        // prevent the router from re-rendering when the initial url or config changes
        true
    }
}

/// A component that renders the current route.
pub fn Router<R: Routable + Clone>(props: RouterProps<R>) -> Element {
    use crate::{outlet::OutletContext, RouterContext};

    use_hook(|| {
        provide_router_context(RouterContext::new(props.config.call(())));
    });

    #[cfg(feature = "streaming")]
    dioxus_hooks::use_after_suspense_resolved(|| {
        dioxus_fullstack_hooks::commit_initial_chunk();
    });

    use_hook(|| {
        provide_context(OutletContext::<R>::new());
    });

    rsx! { Outlet::<R> {} }
}

/// A component that navigates to a new route.
#[component]
pub fn Route(to: String, children: Element) -> Element {
    todo!()
}
