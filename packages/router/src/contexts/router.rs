use std::{
    collections::HashSet,
    error::Error,
    fmt::Display,
    sync::{Arc, Mutex},
};

use dioxus_core::{provide_context, Element, ReactiveContext, ScopeId};
use dioxus_history::history;
use dioxus_signals::{CopyValue, ReadableExt, Signal, WritableExt};

use crate::{
    components::child_router::consume_child_route_mapping, navigation::NavigationTarget,
    routable::Routable, router_cfg::RouterConfig, SiteMapSegment,
};

/// An error that is thrown when the router fails to parse a route
#[derive(Debug, Clone)]
pub struct ParseRouteError {
    message: String,
}

impl Error for ParseRouteError {}
impl Display for ParseRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

/// This context is set in the root of the virtual dom if there is a router present.
#[derive(Clone, Copy)]
struct RootRouterContext(Signal<Option<RouterContext>>);

/// Try to get the router that was created closest to the root of the virtual dom. This may be called outside of the router.
///
/// This will return `None` if there is no router present or the router has not been created yet.
pub fn root_router() -> Option<RouterContext> {
    let rt = dioxus_core::Runtime::current();

    if let Some(ctx) = rt.consume_context::<RootRouterContext>(ScopeId::ROOT) {
        ctx.0.cloned()
    } else {
        rt.provide_context(
            ScopeId::ROOT,
            RootRouterContext(Signal::new_in_scope(None, ScopeId::ROOT)),
        );
        None
    }
}

pub(crate) fn provide_router_context(ctx: RouterContext) {
    if root_router().is_none() {
        dioxus_core::Runtime::current().provide_context(
            ScopeId::ROOT,
            RootRouterContext(Signal::new_in_scope(Some(ctx), ScopeId::ROOT)),
        );
    }
    provide_context(ctx);
}

/// An error that can occur when navigating.
#[derive(Debug, Clone)]
pub struct ExternalNavigationFailure(pub String);

/// A function the router will call after every routing update.
pub(crate) type RoutingCallback<R> =
    Arc<dyn Fn(GenericRouterContext<R>) -> Option<NavigationTarget<R>>>;
pub(crate) type AnyRoutingCallback = Arc<dyn Fn(RouterContext) -> Option<NavigationTarget>>;

struct RouterContextInner {
    unresolved_error: Option<ExternalNavigationFailure>,

    subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
    routing_callback: Option<AnyRoutingCallback>,

    failure_external_navigation: fn() -> Element,

    internal_route: fn(&str) -> bool,

    site_map: &'static [SiteMapSegment],
}

impl RouterContextInner {
    fn update_subscribers(&self) {
        for &id in self.subscribers.lock().unwrap().iter() {
            id.mark_dirty();
        }
    }

    fn subscribe_to_current_context(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.subscribers.clone());
        }
    }

    fn external(&mut self, external: String) -> Option<ExternalNavigationFailure> {
        match history().external(external.clone()) {
            true => None,
            false => {
                let failure = ExternalNavigationFailure(external);
                self.unresolved_error = Some(failure.clone());

                self.update_subscribers();

                Some(failure)
            }
        }
    }
}

/// A collection of router data that manages all routing functionality.
#[derive(Clone, Copy)]
pub struct RouterContext {
    inner: CopyValue<RouterContextInner>,
}

impl RouterContext {
    pub(crate) fn new<R: Routable + 'static>(cfg: RouterConfig<R>) -> Self {
        let subscribers = Arc::new(Mutex::new(HashSet::new()));
        let mapping = consume_child_route_mapping();

        let myself = RouterContextInner {
            unresolved_error: None,
            subscribers: subscribers.clone(),
            routing_callback: cfg.on_update.map(|update| {
                Arc::new(move |ctx| {
                    let ctx = GenericRouterContext {
                        inner: ctx,
                        _marker: std::marker::PhantomData,
                    };
                    update(ctx).map(|t| match t {
                        NavigationTarget::Internal(r) => match mapping.as_ref() {
                            Some(mapping) => {
                                NavigationTarget::Internal(mapping.format_route_as_root_route(r))
                            }
                            None => NavigationTarget::Internal(r.to_string()),
                        },
                        NavigationTarget::External(s) => NavigationTarget::External(s),
                    })
                }) as Arc<dyn Fn(RouterContext) -> Option<NavigationTarget>>
            }),

            failure_external_navigation: cfg.failure_external_navigation,

            internal_route: |route| R::from_str(route).is_ok(),

            site_map: R::SITE_MAP,
        };

        let history = history();

        // set the updater
        history.updater(Arc::new(move || {
            for &rc in subscribers.lock().unwrap().iter() {
                rc.mark_dirty();
            }
        }));

        let myself = Self {
            inner: CopyValue::new_in_scope(myself, ScopeId::ROOT),
        };

        // If the current route is different from the one in the browser, replace the current route
        let current_route: R = myself.current();

        if current_route.to_string() != history.current_route() {
            myself.replace(current_route);
        }

        myself
    }

    /// Check if the router is running in a liveview context
    /// We do some slightly weird things for liveview because of the network boundary
    pub(crate) fn include_prevent_default(&self) -> bool {
        history().include_prevent_default()
    }

    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        history().can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        history().can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        history().go_back();
        self.change_route();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        history().go_forward();
        self.change_route();
    }

    pub(crate) fn push_any(&self, target: NavigationTarget) -> Option<ExternalNavigationFailure> {
        {
            let mut write = self.inner.write_unchecked();
            match target {
                NavigationTarget::Internal(p) => history().push(p),
                NavigationTarget::External(e) => return write.external(e),
            }
        }

        self.change_route()
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: impl Into<NavigationTarget>) -> Option<ExternalNavigationFailure> {
        let target = target.into();
        {
            let mut write = self.inner.write_unchecked();
            match target {
                NavigationTarget::Internal(p) => {
                    let history = history();
                    history.push(p)
                }
                NavigationTarget::External(e) => return write.external(e),
            }
        }

        self.change_route()
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(
        &self,
        target: impl Into<NavigationTarget>,
    ) -> Option<ExternalNavigationFailure> {
        let target = target.into();
        {
            let mut state = self.inner.write_unchecked();
            match target {
                NavigationTarget::Internal(p) => {
                    let history = history();
                    history.replace(p)
                }
                NavigationTarget::External(e) => return state.external(e),
            }
        }

        self.change_route()
    }

    /// The route that is currently active.
    pub fn current<R: Routable>(&self) -> R {
        let absolute_route = self.full_route_string();
        // If this is a child route, map the absolute route to the child route before parsing
        let mapping = consume_child_route_mapping::<R>();
        let route = match mapping.as_ref() {
            Some(mapping) => mapping
                .parse_route_from_root_route(&absolute_route)
                .ok_or_else(|| "Failed to parse route".to_string()),
            None => {
                R::from_str(&absolute_route).map_err(|err| format!("Failed to parse route {err}"))
            }
        };

        match route {
            Ok(route) => route,
            Err(err) => {
                dioxus_core::throw_error(ParseRouteError { message: err });
                "/".parse().unwrap_or_else(|err| panic!("{err}"))
            }
        }
    }

    /// The full route that is currently active. If this is called from inside a child router, this will always return the parent's view of the route.
    pub fn full_route_string(&self) -> String {
        let inner = self.inner.read();
        inner.subscribe_to_current_context();
        let history = history();
        history.current_route()
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        let history = history();
        history.current_prefix()
    }

    /// Clear any unresolved errors
    pub fn clear_error(&self) {
        let mut write_inner = self.inner.write_unchecked();
        write_inner.unresolved_error = None;

        write_inner.update_subscribers();
    }

    /// Get the site map of the router.
    pub fn site_map(&self) -> &'static [SiteMapSegment] {
        self.inner.read().site_map
    }

    pub(crate) fn render_error(&self) -> Option<Element> {
        let inner_write = self.inner.write_unchecked();
        inner_write.subscribe_to_current_context();
        inner_write
            .unresolved_error
            .as_ref()
            .map(|_| (inner_write.failure_external_navigation)())
    }

    fn change_route(&self) -> Option<ExternalNavigationFailure> {
        let self_read = self.inner.read();
        if let Some(callback) = &self_read.routing_callback {
            let myself = *self;
            let callback = callback.clone();
            drop(self_read);
            if let Some(new) = callback(myself) {
                let mut self_write = self.inner.write_unchecked();
                match new {
                    NavigationTarget::Internal(p) => {
                        let history = history();
                        history.replace(p)
                    }
                    NavigationTarget::External(e) => return self_write.external(e),
                }
            }
        }

        self.inner.read().update_subscribers();

        None
    }

    pub(crate) fn internal_route(&self, route: &str) -> bool {
        (self.inner.read().internal_route)(route)
    }
}

/// This context is set to the RouterConfig on_update method
pub struct GenericRouterContext<R> {
    inner: RouterContext,
    _marker: std::marker::PhantomData<R>,
}

impl<R> GenericRouterContext<R>
where
    R: Routable,
{
    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.inner.can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.inner.can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        self.inner.go_back();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        self.inner.go_forward();
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(
        &self,
        target: impl Into<NavigationTarget<R>>,
    ) -> Option<ExternalNavigationFailure> {
        self.inner.push(target.into())
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(
        &self,
        target: impl Into<NavigationTarget<R>>,
    ) -> Option<ExternalNavigationFailure> {
        self.inner.replace(target.into())
    }

    /// The route that is currently active.
    pub fn current(&self) -> R
    where
        R: Clone,
    {
        self.inner.current()
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        self.inner.prefix()
    }

    /// Clear any unresolved errors
    pub fn clear_error(&self) {
        self.inner.clear_error()
    }
}
