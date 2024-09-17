use core::panic;
use std::{
    any::Any,
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

use super::{ExternalNavigationFailure, RouterContext};

pub struct GenericRouterContext<R> {
    inner: RouterContext,
    _marker: std::marker::PhantomData<R>,
}

impl<R: Routable> GenericRouterContext<R> {
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
        todo!()
        // self.inner.push(target.into())
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(
        &self,
        target: impl Into<NavigationTarget<R>>,
    ) -> Option<ExternalNavigationFailure> {
        todo!()
        // self.inner.replace(target.into())
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
