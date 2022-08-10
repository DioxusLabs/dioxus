use std::{collections::BTreeMap, fmt::Debug};

use dioxus::prelude::*;

use crate::navigation::NavigationTarget;

/// The content of a [`Route`] or [`ParameterRoute`].
///
/// [`Route`]: super::Route
/// [`ParameterRoute`]: super::ParameterRoute
#[derive(Clone)]
pub enum RouteContent {
    /// No content.
    ///
    /// Can be used to make a route transparent and have its nested routes be nested one level less
    /// deep.
    Empty,
    /// A single component.
    Component(Component),
    /// One main and several side components.
    Multi(Component, Vec<(&'static str, Component)>),
    /// Causes a redirect when the route is matched.
    ///
    /// Redirects are performed as a _replace_ operation. This means that the original path won't be
    /// part of the history.
    ///
    /// Be careful to not create an infinite loop. While certain [`HistoryProvider`]s may stop after
    ///  a threshold is reached, others (like [`MemoryHistory`]) will not.
    ///
    /// [`HistoryProvider`]: crate::history::HistoryProvider
    /// [`MemoryHistory`]: crate::history::MemoryHistory
    Redirect(NavigationTarget),
}

impl RouteContent {
    /// Add the contained content to `components` or return a redirect.
    #[must_use]
    pub(crate) fn add_to_list(
        &self,
        components: &mut (Vec<Component>, BTreeMap<&'static str, Vec<Component>>),
    ) -> Option<NavigationTarget> {
        match self {
            RouteContent::Empty => {}
            RouteContent::Component(comp) => components.0.push(*comp),
            RouteContent::Multi(main, side) => {
                components.0.push(*main);
                for (name, comp) in side {
                    if let Some(x) = components.1.get_mut(name) {
                        x.push(*comp);
                    } else {
                        components.1.insert(name, vec![*comp]);
                    }
                }
            }
            RouteContent::Redirect(t) => return Some(t.clone()),
        }

        None
    }

    /// Returns [`true`] if the route content is [`Empty`].
    ///
    /// [`Empty`]: RouteContent::Empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

// [`Component`] (in [`Component`] and [`Multi`]) doesn't implement [`Debug`]
impl Debug for RouteContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Component(_) => f.debug_tuple("Component").finish(),
            Self::Multi(_, _) => f.debug_tuple("Multi").finish(),
            Self::Redirect(arg0) => f.debug_tuple("Redirect").field(arg0).finish(),
        }
    }
}

impl Default for RouteContent {
    fn default() -> Self {
        Self::Empty
    }
}

impl From<()> for RouteContent {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}

impl From<Component> for RouteContent {
    fn from(c: Component) -> Self {
        Self::Component(c)
    }
}

impl<T: Into<NavigationTarget>> From<T> for RouteContent {
    fn from(nt: T) -> Self {
        Self::Redirect(nt.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_to_list_rc_none() {
        let mut components = (Vec::new(), BTreeMap::new());
        let content = RouteContent::Empty;

        let res = content.add_to_list(&mut components);

        assert!(res.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
    }

    #[test]
    fn add_to_list_rc_component() {
        let mut components = (Vec::new(), BTreeMap::new());
        let content = RouteContent::Component(TestComponent);

        let res = content.add_to_list(&mut components);

        assert!(res.is_none());
        assert_eq!(components.0.len(), 1);
        // TODO: check if contained component is the correct one
        assert!(components.1.is_empty());
    }

    #[test]
    fn add_to_list_rc_multi() {
        let mut components = (Vec::new(), BTreeMap::new());
        let content = RouteContent::Multi(TestComponent, vec![("test", TestComponent)]);

        let res = content.add_to_list(&mut components);

        assert!(res.is_none());
        assert_eq!(components.0.len(), 1);
        // TODO: check if contained component is the correct one
        assert_eq!(components.1.len(), 1);
        assert!(components.1.contains_key("test"));
        // TODO: check if contained component is the correct one
    }

    #[test]
    fn add_to_list_rc_redirect() {
        let nt = NavigationTarget::InternalTarget(String::from("test"));
        let mut components = (Vec::new(), BTreeMap::new());
        let content = RouteContent::Redirect(nt.clone());

        let res = content.add_to_list(&mut components);

        assert!(res.is_some());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
    }

    #[allow(non_snake_case)]
    fn TestComponent(_: Scope) -> Element {
        unimplemented!()
    }
}
