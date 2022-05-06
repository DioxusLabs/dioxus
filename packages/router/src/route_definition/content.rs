use std::collections::BTreeMap;

use dioxus_core::Component;

use crate::navigation::NavigationTarget;

/// The content of a [`Route`] or [`DynamicRoute`].
///
/// [`Route`]: super::Route
/// [`DynamicRoute`]: super::DynamicRoute
#[derive(Clone)]
pub enum RouteContent {
    /// Indicates the absence of content.
    ///
    /// When used for an `index` it marks that no index content exists.
    ///
    /// When used for a `fixed` or `dynamic` no components will be rendered. If a nested route is
    /// matched its content will be rendered in the outlet where this segments content would be
    /// rendered.
    RcNone,
    /// A single component.
    RcComponent(Component),
    /// One main and several side components.
    RcMulti(Component, Vec<(&'static str, Component)>),
    /// Causes a redirect when the route is matched.
    ///
    /// Redirects are performed as a _replace_ operation. This means that the original path won't be
    /// part of the history.
    ///
    /// Be careful to not create an infinite loop. While certain [HistoryProvider]s may stop after a
    /// threshold is reached, others (like [MemoryHistoryProvider]) will not.
    ///
    /// [HistoryProvider]: crate::history::HistoryProvider
    /// [MemoryHistoryProvider]: crate::history::MemoryHistoryProvider
    RcRedirect(NavigationTarget),
}

impl RouteContent {
    #[must_use]
    pub(crate) fn add_to_list(
        &self,
        components: &mut (Vec<Component>, BTreeMap<&'static str, Vec<Component>>),
    ) -> Option<NavigationTarget> {
        match self {
            RouteContent::RcNone => {}
            RouteContent::RcComponent(comp) => components.0.push(*comp),
            RouteContent::RcMulti(main, side) => {
                components.0.push(*main);
                for (name, comp) in side {
                    if let Some(x) = components.1.get_mut(name) {
                        x.push(*comp);
                    } else {
                        components.1.insert(name, vec![*comp]);
                    }
                }
            }
            RouteContent::RcRedirect(t) => return Some(t.clone()),
        }

        None
    }

    /// Returns [`true`] if the route content is [`RcNone`].
    ///
    /// [`RcNone`]: RouteContent::RcNone
    #[must_use]
    pub fn is_rc_none(&self) -> bool {
        matches!(self, Self::RcNone)
    }
}

impl Default for RouteContent {
    fn default() -> Self {
        Self::RcNone
    }
}
