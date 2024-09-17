use std::{
    collections::HashSet,
    rc::Rc,
    sync::{Arc, RwLock},
};

use dioxus_document::DocumentContext;
use dioxus_lib::prelude::*;

use crate::{
    navigation::NavigationTarget,
    prelude::{IntoRoutable, SiteMapSegment},
    routable::Routable,
    router_cfg::RouterConfig,
};

use super::generic_router::GenericRouterContext;

/// A collection of router data that manages all routing functionality.
#[derive(Clone, Copy)]
pub struct RouterContext {
    inner: CopyValue<RouterContextInner>,
}

struct RouterContextInner {
    basepath: Option<String>,

    runtime: Rc<Runtime>,

    document: DocumentContext,

    unresolved_error: Option<ExternalNavigationFailure>,

    subscribers: Arc<RwLock<HashSet<ScopeId>>>,

    failure_external_navigation: fn() -> Element,

    site_map: &'static [SiteMapSegment],
    // any_route_to_string: fn(&dyn Any) -> String,
    // routing_callback: Option<AnyRoutingCallback>,
}

impl RouterContext {
    pub(crate) fn new<R: Routable + 'static>(mut cfg: RouterConfig<R>) -> Self {
        let subscribers = Arc::new(RwLock::new(HashSet::new()));

        let mut myself = RouterContextInner {
            basepath: Default::default(),
            unresolved_error: None,
            subscribers: subscribers.clone(),
            document: todo!(),
            failure_external_navigation: cfg.failure_external_navigation,
            site_map: todo!(),
            runtime: todo!(),
            // history: cfg.take_history(),

            // subscriber_update,

            // routing_callback: cfg.on_update.map(|update| {
            //     Arc::new(move |ctx| {
            //         let ctx = GenericRouterContext {
            //             inner: ctx,
            //             _marker: std::marker::PhantomData,
            //         };
            //         update(ctx).map(|t| match t {
            //             NavigationTarget::Internal(r) => {
            //                 NavigationTarget::Internal(Rc::new(r) as Rc<dyn Any>)
            //             }
            //             NavigationTarget::External(s) => NavigationTarget::External(s),
            //         })
            //     })
            //         as Arc<dyn Fn(RouterContext) -> Option<NavigationTarget<Rc<dyn Any>>>>
            // }),
            // any_route_to_string: |route| {
            // todo!()
            // route
            //     .downcast_ref::<R>()
            //     .unwrap_or_else(|| {
            //         panic!(
            //             "Route is not of the expected type: {}\n found typeid: {:?}\n expected typeid: {:?}",
            //             std::any::type_name::<R>(),
            //             route.type_id(),
            //             std::any::TypeId::of::<R>()
            //         )
            //     })
            //     .to_string()
            // },
            // site_map: R::SITE_MAP,
            // routing_callback: todo!(),
        };

        // set the updater
        {
            let rt = myself.runtime.clone();
            myself.document.updater(Arc::new(move || {
                for &id in subscribers.read().unwrap().iter() {
                    rt.mark_dirty(id);
                }
            }));
        }

        Self {
            inner: CopyValue::new_in_scope(myself, ScopeId::ROOT),
        }
    }

    /// Check if the router is running in a liveview context
    /// We do some slightly weird things for liveview because of the network boundary
    pub fn is_synchronous(&self) -> bool {
        self.inner.read().document.is_synchronous()
    }

    pub(crate) fn route_from_str<R: Routable>(&self, route: &str) -> Result<R, String> {
        todo!()
        // self.inner.read().history.parse_route(route)
    }

    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.inner.read().document.can_go_back()
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.inner.read().document.can_go_forward()
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        {
            self.inner.write_unchecked().document.go_back();
        }

        self.change_route();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        {
            self.inner.write_unchecked().document.go_forward();
        }

        self.change_route();
    }

    pub(crate) fn push_any<R: Routable>(
        &self,
        target: NavigationTarget<R>,
    ) -> Option<ExternalNavigationFailure> {
        match target {
            NavigationTarget::Internal(p) => self
                .inner
                .write_unchecked()
                .document
                .push_route(p.serialize()),
            NavigationTarget::External(e) => return self.navigate_external(e),
        }

        self.change_route()
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        todo!()
        // let target = self.resolve_into_routable(target.into());
        // {
        //     let mut write = self.inner.write_unchecked();
        //     match target {
        //         NavigationTarget::Internal(p) => write.history.push_route(p),
        //         NavigationTarget::External(e) => return write.external(e),
        //     }
        // }

        // self.change_route()
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(&self, target: impl Into<IntoRoutable>) -> Option<ExternalNavigationFailure> {
        todo!()
        // let target = self.resolve_into_routable(target.into());

        // {
        //     let mut state = self.inner.write_unchecked();
        //     match target {
        //         NavigationTarget::Internal(p) => state.history.replace_route(p),
        //         NavigationTarget::External(e) => return state.external(e),
        //     }
        // }

        // self.change_route()
    }

    /// The route that is currently active.
    pub fn current<R: Routable>(&self) -> R {
        todo!()
        // self.inner
        //     .read()
        //     .history
        //     .current_route()
        //     .parse()
        //     .unwrap_or_else(|err| panic!("Failed to parse route"))
    }

    /// The route that is currently active.
    pub fn current_route_string(&self) -> String {
        self.inner.read_unchecked().document.current_route()
    }

    // pub(crate) fn any_route_to_string(&self, route: &dyn Any) -> String {
    //     (self.inner.read().any_route_to_string)(route)
    // }

    pub(crate) fn resolve_into_routable<R: Routable>(
        &self,
        into_routable: IntoRoutable,
    ) -> NavigationTarget<R> {
        todo!()
        // match into_routable {
        //     IntoRoutable::FromStr(url) => {
        //         let parsed_route: NavigationTarget<Rc<dyn Any>> = match self.route_from_str(&url) {
        //             Ok(route) => NavigationTarget::Internal(route),
        //             Err(_) => NavigationTarget::External(url),
        //         };
        //         parsed_route
        //     }
        //     IntoRoutable::Route(route) => NavigationTarget::Internal(route),
        // }
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        self.inner.read().basepath.clone()
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
        {
            let mut write_inner = self.inner.write_unchecked();
            write_inner.unresolved_error = None;
        }
        self.update_subscribers();
    }

    /// Get the site map of the router.
    pub fn site_map(&self) -> &'static [SiteMapSegment] {
        self.inner.read().site_map
    }

    pub(crate) fn render_error(&self) -> Option<Element> {
        let inner_read = self.inner.write_unchecked();
        inner_read
            .unresolved_error
            .as_ref()
            .map(|_| (inner_read.failure_external_navigation)())
    }

    fn change_route(&self) -> Option<ExternalNavigationFailure> {
        todo!()
        // let self_read = self.inner.read();
        // if let Some(callback) = &self_read.routing_callback {
        //     let myself = *self;
        //     let callback = callback.clone();
        //     drop(self_read);
        //     if let Some(new) = callback(myself) {
        //         let mut self_write = self.inner.write_unchecked();
        //         match new {
        //             NavigationTarget::Internal(p) => self_write.history.replace_route(p),
        //             NavigationTarget::External(e) => return self_write.external(e),
        //         }
        //     }
        // }

        // self.inner.read().update_subscribers();

        // None
    }

    fn update_subscribers(&self) {
        let inner = self.inner.read_unchecked();
        for &id in inner.subscribers.read().unwrap().iter() {
            inner.runtime.mark_dirty(id)
        }
    }

    fn navigate_external(&self, external: String) -> Option<ExternalNavigationFailure> {
        let failure = {
            let mut myself = self.inner.write_unchecked();
            match myself.document.navigate_external(external.clone()) {
                true => None,
                false => {
                    let failure = ExternalNavigationFailure(external);
                    myself.unresolved_error = Some(failure.clone());
                    Some(failure)
                }
            }
        };

        if failure.is_some() {
            self.update_subscribers();
        }

        failure
    }
}

/// An error that can occur when navigating.
#[derive(Debug, Clone)]
pub struct ExternalNavigationFailure(pub String);

/// A function the router will call after every routing update.
pub(crate) type RoutingCallback<R> =
    Arc<dyn Fn(GenericRouterContext<R>) -> Option<NavigationTarget<R>>>;

pub(crate) type AnyRoutingCallback = Arc<dyn Fn(RouterContext) -> Option<NavigationTarget<String>>>;
// Arc<dyn Fn(RouterContext) -> Option<NavigationTarget<Rc<dyn Any>>>>;
