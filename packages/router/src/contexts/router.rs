use std::{
    collections::HashSet,
    sync::{Arc, RwLock, RwLockWriteGuard},
};

use dioxus::prelude::*;

use crate::{
    history::HistoryProvider, navigation::NavigationTarget, routable::Routable,
    router_cfg::RouterConfig,
};

/// An error that can occur when navigating.
#[derive(Debug, Clone)]
pub struct ExternalNavigationFailure(String);

/// A function the router will call after every routing update.
pub(crate) type RoutingCallback<R> =
    Arc<dyn Fn(GenericRouterContext<R>) -> Option<NavigationTarget<R>>>;

struct MutableRouterState<R>
where
    R: Routable,
{
    /// The current prefix.
    prefix: Option<String>,

    history: Box<dyn HistoryProvider<R>>,

    unresolved_error: Option<ExternalNavigationFailure>,
}

/// A collection of router data that manages all routing functionality.
pub struct GenericRouterContext<R>
where
    R: Routable,
{
    state: Arc<RwLock<MutableRouterState<R>>>,

    subscribers: Arc<RwLock<HashSet<ScopeId>>>,
    subscriber_update: Arc<dyn Fn(ScopeId)>,
    routing_callback: Option<RoutingCallback<R>>,

    failure_external_navigation: fn(Scope) -> Element,
}

impl<R: Routable> Clone for GenericRouterContext<R> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            subscribers: self.subscribers.clone(),
            subscriber_update: self.subscriber_update.clone(),
            routing_callback: self.routing_callback.clone(),
            failure_external_navigation: self.failure_external_navigation,
        }
    }
}

impl<R> GenericRouterContext<R>
where
    R: Routable,
{
    pub(crate) fn new(
        mut cfg: RouterConfig<R>,
        mark_dirty: Arc<dyn Fn(ScopeId) + Sync + Send>,
    ) -> Self
    where
        R: Clone,
        <R as std::str::FromStr>::Err: std::fmt::Display,
    {
        let state = Arc::new(RwLock::new(MutableRouterState {
            prefix: Default::default(),
            history: cfg.take_history(),
            unresolved_error: None,
        }));

        let subscriber_update = mark_dirty.clone();
        let subscribers = Arc::new(RwLock::new(HashSet::new()));

        let myself = Self {
            state,
            subscribers: subscribers.clone(),
            subscriber_update,

            routing_callback: cfg.on_update,

            failure_external_navigation: cfg.failure_external_navigation,
        };

        // set the updater
        {
            let mut state = myself.state.write().unwrap();
            state.history.updater(Arc::new(move || {
                for &id in subscribers.read().unwrap().iter() {
                    (mark_dirty)(id);
                }
            }));
        }

        myself
    }

    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.state.read().unwrap().history.can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.state.read().unwrap().history.can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        {
            self.state.write().unwrap().history.go_back();
        }

        self.change_route();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        {
            self.state.write().unwrap().history.go_forward();
        }

        self.change_route();
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(
        &self,
        target: impl Into<NavigationTarget<R>>,
    ) -> Option<ExternalNavigationFailure> {
        let target = target.into();
        match target {
            NavigationTarget::Internal(p) => {
                let mut state = self.state_mut();
                state.history.push(p)
            }
            NavigationTarget::External(e) => return self.external(e),
        }

        self.change_route()
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(
        &self,
        target: impl Into<NavigationTarget<R>>,
    ) -> Option<ExternalNavigationFailure> {
        let target = target.into();

        {
            let mut state = self.state_mut();
            match target {
                NavigationTarget::Internal(p) => state.history.replace(p),
                NavigationTarget::External(e) => return self.external(e),
            }
        }

        self.change_route()
    }

    /// The route that is currently active.
    pub fn current(&self) -> R
    where
        R: Clone,
    {
        self.state.read().unwrap().history.current_route()
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        self.state.read().unwrap().prefix.clone()
    }

    fn external(&self, external: String) -> Option<ExternalNavigationFailure> {
        let mut state = self.state_mut();
        match state.history.external(external.clone()) {
            true => None,
            false => {
                let failure = ExternalNavigationFailure(external);
                state.unresolved_error = Some(failure.clone());

                self.update_subscribers();

                Some(failure)
            }
        }
    }

    fn state_mut(&self) -> RwLockWriteGuard<MutableRouterState<R>> {
        self.state.write().unwrap()
    }

    /// Manually subscribe to the current route
    pub fn subscribe(&self, id: ScopeId) {
        self.subscribers.write().unwrap().insert(id);
    }

    /// Manually unsubscribe from the current route
    pub fn unsubscribe(&self, id: ScopeId) {
        self.subscribers.write().unwrap().remove(&id);
    }

    fn update_subscribers(&self) {
        for &id in self.subscribers.read().unwrap().iter() {
            (self.subscriber_update)(id);
        }
    }

    /// Clear any unresolved errors
    pub fn clear_error(&self) {
        self.state.write().unwrap().unresolved_error = None;

        self.update_subscribers();
    }

    pub(crate) fn render_error<'a>(&self, cx: Scope<'a>) -> Element<'a> {
        self.state
            .read()
            .unwrap()
            .unresolved_error
            .as_ref()
            .and_then(|_| (self.failure_external_navigation)(cx))
    }

    fn change_route(&self) -> Option<ExternalNavigationFailure> {
        if let Some(callback) = &self.routing_callback {
            let myself = self.clone();
            if let Some(new) = callback(myself) {
                let mut state = self.state_mut();
                match new {
                    NavigationTarget::Internal(p) => state.history.replace(p),
                    NavigationTarget::External(e) => return self.external(e),
                }
            }
        }

        self.update_subscribers();

        None
    }
}
