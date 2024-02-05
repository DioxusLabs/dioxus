use std::{
    any::Any,
    collections::HashSet,
    rc::Rc,
    sync::{Arc, RwLock},
};

use dioxus_lib::prelude::*;

use crate::{
    navigation::NavigationTarget,
    prelude::{AnyHistoryProvider, IntoRoutable},
    routable::Routable,
    router_cfg::RouterConfig,
};

/// An error that can occur when navigating.
#[derive(Debug, Clone)]
pub struct ExternalNavigationFailure(pub String);

/// A function the router will call after every routing update.
pub(crate) type RoutingCallback<R> =
    Arc<dyn Fn(GenericRouterContext<R>) -> Option<NavigationTarget<R>>>;
pub(crate) type AnyRoutingCallback =
    Arc<dyn Fn(RouterContext) -> Option<NavigationTarget<Rc<dyn Any>>>>;

struct RouterContextInner {
    /// The current prefix.
    prefix: Option<String>,

    history: Box<dyn AnyHistoryProvider>,

    unresolved_error: Option<ExternalNavigationFailure>,

    subscribers: Arc<RwLock<HashSet<ScopeId>>>,
    subscriber_update: Arc<dyn Fn(ScopeId)>,
    routing_callback: Option<AnyRoutingCallback>,

    failure_external_navigation: fn() -> Element,

    any_route_to_string: fn(&dyn Any) -> String,
}

impl RouterContextInner {
    fn update_subscribers(&self) {
        let update = &self.subscriber_update;
        for &id in self.subscribers.read().unwrap().iter() {
            update(id);
        }
    }

    fn external(&mut self, external: String) -> Option<ExternalNavigationFailure> {
        match self.history.external(external.clone()) {
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
    pub(crate) fn new<R: Routable + 'static>(
        mut cfg: RouterConfig<R>,
        mark_dirty: Arc<dyn Fn(ScopeId) + Sync + Send>,
    ) -> Self
    where
        R: Clone,
        <R as std::str::FromStr>::Err: std::fmt::Display,
    {
        let subscriber_update = mark_dirty.clone();
        let subscribers = Arc::new(RwLock::new(HashSet::new()));

        let mut myself = RouterContextInner {
            prefix: Default::default(),
            history: cfg.take_history(),
            unresolved_error: None,
            subscribers: subscribers.clone(),
            subscriber_update,

            routing_callback: cfg.on_update.map(|update| {
                Arc::new(move |ctx| {
                    let ctx = GenericRouterContext {
                        inner: ctx,
                        _marker: std::marker::PhantomData,
                    };
                    update(ctx).map(|t| match t {
                        NavigationTarget::Internal(r) => {
                            NavigationTarget::Internal(Rc::new(r) as Rc<dyn Any>)
                        }
                        NavigationTarget::External(s) => NavigationTarget::External(s),
                    })
                })
                    as Arc<dyn Fn(RouterContext) -> Option<NavigationTarget<Rc<dyn Any>>>>
            }),

            failure_external_navigation: cfg.failure_external_navigation,

            any_route_to_string: |route| {
                route
                    .downcast_ref::<R>()
                    .unwrap_or_else(|| {
                        panic!(
                            "Route is not of the expected type: {}\n found typeid: {:?}\n expected typeid: {:?}",
                            std::any::type_name::<R>(),
                            route.type_id(),
                            std::any::TypeId::of::<R>()
                        )
                    })
                    .to_string()
            },
        };

        // set the updater
        {
            myself.history.updater(Arc::new(move || {
                for &id in subscribers.read().unwrap().iter() {
                    (mark_dirty)(id);
                }
            }));
        }

        Self {
            inner: CopyValue::new_in_scope(myself, ScopeId::ROOT),
        }
    }

    pub(crate) fn route_from_str(&self, route: &str) -> Result<Rc<dyn Any>, String> {
        self.inner.read().history.parse_route(route)
    }

    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.inner.read().history.can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.inner.read().history.can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        {
            self.inner.clone().write().history.go_back();
        }

        self.change_route();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        {
            self.inner.clone().write().history.go_forward();
        }

        self.change_route();
    }

    pub(crate) fn push_any(
        &self,
        target: NavigationTarget<Rc<dyn Any>>,
    ) -> Option<ExternalNavigationFailure> {
        {
            let mut write = self.inner.clone().write();
            match target {
                NavigationTarget::Internal(p) => write.history.push(p),
                NavigationTarget::External(e) => return write.external(e),
            }
        }

        self.change_route()
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        let target = self.resolve_into_routable(target.into());
        {
            let mut write = self.inner.clone().write();
            match target {
                NavigationTarget::Internal(p) => write.history.push(p),
                NavigationTarget::External(e) => return write.external(e),
            }
        }

        self.change_route()
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        let target = self.resolve_into_routable(target.into());

        {
            let mut state = self.inner.clone().write();
            match target {
                NavigationTarget::Internal(p) => state.history.replace(p),
                NavigationTarget::External(e) => return state.external(e),
            }
        }

        self.change_route()
    }

    /// The route that is currently active.
    pub fn current<R: Routable>(&self) -> R {
        self.inner
            .read()
            .history
            .current_route()
            .downcast::<R>()
            .unwrap()
            .as_ref()
            .clone()
    }

    /// The route that is currently active.
    pub fn current_route_string(&self) -> String {
        self.any_route_to_string(&*self.inner.read().history.current_route())
    }

    pub(crate) fn any_route_to_string(&self, route: &dyn Any) -> String {
        (self.inner.read().any_route_to_string)(route)
    }

    pub(crate) fn resolve_into_routable(
        &self,
        into_routable: IntoRoutable,
    ) -> NavigationTarget<Rc<dyn Any>> {
        match into_routable {
            IntoRoutable::FromStr(url) => {
                let parsed_route: NavigationTarget<Rc<dyn Any>> = match self.route_from_str(&url) {
                    Ok(route) => NavigationTarget::Internal(route),
                    Err(_) => NavigationTarget::External(url),
                };
                parsed_route
            }
            IntoRoutable::Route(route) => NavigationTarget::Internal(route),
        }
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        self.inner.read().prefix.clone()
    }

    /// Manually subscribe to the current route
    pub fn subscribe(&self, id: ScopeId) {
        self.inner.read().subscribers.write().unwrap().insert(id);
    }

    /// Manually unsubscribe from the current route
    pub fn unsubscribe(&self, id: ScopeId) {
        self.inner.read().subscribers.write().unwrap().remove(&id);
    }

    /// Clear any unresolved errors
    pub fn clear_error(&self) {
        let mut write_inner = self.inner.clone().write();
        write_inner.unresolved_error = None;

        write_inner.update_subscribers();
    }

    pub(crate) fn render_error(&self) -> Element {
        let inner_read = self.inner.clone().write();
        inner_read
            .unresolved_error
            .as_ref()
            .and_then(|_| (inner_read.failure_external_navigation)())
    }

    fn change_route(&self) -> Option<ExternalNavigationFailure> {
        let self_read = self.inner.read();
        if let Some(callback) = &self_read.routing_callback {
            let myself = *self;
            let callback = callback.clone();
            drop(self_read);
            if let Some(new) = callback(myself) {
                let mut self_write = self.inner.clone().write();
                match new {
                    NavigationTarget::Internal(p) => self_write.history.replace(p),
                    NavigationTarget::External(e) => return self_write.external(e),
                }
            }
        }

        self.inner.read().update_subscribers();

        None
    }
}

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

    /// Manually subscribe to the current route
    pub fn subscribe(&self, id: ScopeId) {
        self.inner.subscribe(id)
    }

    /// Manually unsubscribe from the current route
    pub fn unsubscribe(&self, id: ScopeId) {
        self.inner.unsubscribe(id)
    }

    /// Clear any unresolved errors
    pub fn clear_error(&self) {
        self.inner.clear_error()
    }
}
