use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock, Weak},
};

use dioxus_core::{Component, ScopeId};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::StreamExt;
use log::error;
use urlencoding::decode;

use crate::{
    contexts::RouterContext,
    helpers::construct_named_path,
    history::{HistoryProvider, MemoryHistoryProvider},
    navigation::{InternalNavigationTarget, NamedNavigationSegment},
    route_definition::{DynamicRoute, RouteTarget, Segment},
    state::CurrentRoute,
};

/// A set of messages that the [`RouterService`] can handle.
pub(crate) enum RouterMessage {
    /// Go back a step in the navigation history.
    GoBack,

    /// Go a step forward in the navigation history.
    GoForward,

    /// Push a new path.
    Push(InternalNavigationTarget),

    /// Replace the current path with a new one.
    Replace(InternalNavigationTarget),

    /// Subscribe the specified scope to router updates.
    Subscribe(Arc<ScopeId>),
}

/// The core of the router.
///
/// This combines the [route definitions](crate::route_definition) and a [HistoryProvider] to
/// find what components should be rendered on what level. Also triggers updates of subscribed
/// components when the current route changes.
///
/// The [`RouterService`] can be made to do things by sending it [`RouterMessage`]s via the `tx`
/// field of the [`RouterContext`] it returns when it is constructed.
///
/// The [`RouterService`] provides information about its current state via the `state` field the
/// [`RouterContext`] it returns when it is constructed.
pub(crate) struct RouterService {
    history: Box<dyn HistoryProvider>,
    named_navigation_fallback_path: Option<String>,
    named_routes: Arc<BTreeMap<&'static str, Vec<NamedNavigationSegment>>>,
    routes: Segment,
    rx: UnboundedReceiver<RouterMessage>,
    state: Arc<RwLock<CurrentRoute>>,
    subscribers: Vec<Weak<ScopeId>>,
    update: Arc<dyn Fn(ScopeId)>,
}

impl RouterService {
    /// Create a new [`RouterService`].
    ///
    /// The returned [`RouterService`] and [`RouterContext`] are linked with each other.
    pub(crate) fn new(
        routes: Segment,
        update: Arc<dyn Fn(ScopeId)>,
        named_navigation_fallback_path: Option<String>,
        active_class: Option<String>,
    ) -> (Self, RouterContext) {
        // create channel
        let (tx, rx) = unbounded();

        // create named navigation targets
        let mut named_routes = BTreeMap::new();
        construct_named_targets(&routes, &Vec::new(), &mut named_routes);
        named_routes.insert("root_index", Vec::new());
        let named_routes = Arc::new(named_routes);

        // create state and context
        let state = Arc::new(RwLock::new(CurrentRoute::default()));
        let context = RouterContext {
            active_class,
            tx,
            state: state.clone(),
            named_routes: named_routes.clone(),
        };

        (
            Self {
                history: Box::new(MemoryHistoryProvider::default()),
                named_navigation_fallback_path,
                named_routes,
                routes,
                rx,
                state,
                subscribers: vec![],
                update,
            },
            context,
        )
    }

    /// The routers event loop.
    pub(crate) async fn run(&mut self) {
        // Trigger initial routing. Subscribers rendering before this happens will be updated when
        // the subscription is registered.
        self.update_routing();

        while let Some(x) = self.rx.next().await {
            match x {
                RouterMessage::GoBack => self.history.go_back(),
                RouterMessage::GoForward => self.history.go_forward(),
                RouterMessage::Push(target) => match target {
                    InternalNavigationTarget::IPath(path) => self.history.push(path),
                    InternalNavigationTarget::IName(name, vars) => {
                        let path = construct_named_path(name, &vars, &self.named_routes)
                            .or(self.named_navigation_fallback_path.clone());
                        if let Some(path) = path {
                            self.history.push(path);
                        }
                    }
                },
                RouterMessage::Replace(target) => match target {
                    InternalNavigationTarget::IPath(path) => self.history.replace(path),
                    InternalNavigationTarget::IName(name, vars) => {
                        let path = construct_named_path(name, &vars, &self.named_routes)
                            .or(self.named_navigation_fallback_path.clone());
                        if let Some(path) = path {
                            self.history.replace(path);
                        }
                    }
                },
                RouterMessage::Subscribe(id) => {
                    self.subscribers.push(Arc::downgrade(&id));
                    (self.update)(*id);
                    continue; // no navigation happened
                }
            }

            self.update_routing();
            self.update_subscribers();
        }
    }

    /// Update the current state of the router.
    fn update_routing(&mut self) {
        // prepare varibles
        let mut state = self.state.write().unwrap();
        let CurrentRoute {
            can_go_back,
            can_go_forward,
            components,
            names,
            path: s_path,
            variables,
        } = &mut *state;

        loop {
            let mut path = self.history.current_path().to_string();

            // clear state
            *can_go_back = self.history.can_go_back();
            *can_go_forward = self.history.can_go_forward();
            components.clear();
            names.clear();
            *s_path = path.clone();
            variables.clear();

            // normalize and split path
            path.remove(0);
            if path.ends_with("/") {
                path.remove(path.len() - 1);
            }
            let segments: Vec<_> = path.split("/").collect();

            // handle index on root
            let next = if path.len() == 0 {
                if let Some(target) = &self.routes.index {
                    match target {
                        RouteTarget::TComponent(main, side) => {
                            components.push((*main, BTreeMap::from_iter(side.clone().into_iter())));
                            names.insert("root_index");
                            None
                        }
                        RouteTarget::TRedirect(t) => Some(t.clone()),
                    }
                } else {
                    None
                }
            } else {
                match_segment(&segments, &self.routes, components, names, variables)
            };

            if let Some(target) = next {
                self.history.replace(match target {
                    InternalNavigationTarget::IPath(p) => p,
                    InternalNavigationTarget::IName(name, vars) => {
                        match construct_named_path(name, &vars, &self.named_routes)
                            .or(self.named_navigation_fallback_path.clone())
                        {
                            Some(p) => p,
                            None => break,
                        }
                    }
                })
            } else {
                break;
            }
        }
    }

    /// Trigger an update of all subscribed components.
    ///
    /// Also sorts out the components that have been unmounted since the last update, as well as any
    /// duplicate within the subscribers.
    fn update_subscribers(&mut self) {
        let update = self.update.as_ref();
        let mut ids = Vec::with_capacity(self.subscribers.len());

        self.subscribers.retain(|s| {
            // get rid of unmounted components
            if let Some(s) = s.upgrade() {
                // get rid of duplicates and trigger only one update
                if !ids.contains(&*s) {
                    ids.push(*s);
                    (update)(*s);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });
    }
}

fn construct_named_targets(
    seg: &Segment,
    ancestors: &Vec<NamedNavigationSegment>,
    named: &mut BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) {
    for (path, route) in &seg.fixed {
        // prepare new ancestors
        let mut ancestors = ancestors.clone();
        ancestors.push(NamedNavigationSegment::Fixed(path.to_string()));

        if let Some(seg) = &route.sub {
            construct_named_targets(seg, &ancestors, named);
        }

        if let Some(name) = route.name {
            if named.insert(name, ancestors).is_some() {
                error!(r#"route names must be unique; duplicate name: "{name}""#);
                #[cfg(debug_assertions)]
                panic!(r#"duplicate route name: "{name}""#)
            };
        }
    }

    if let DynamicRoute::Variable {
        name,
        key,
        content: _,
        sub,
    } = &seg.dynamic
    {
        let mut ancestors = ancestors.clone();
        ancestors.push(NamedNavigationSegment::Variable(key));

        if let Some(seg) = sub {
            construct_named_targets(seg, &ancestors, named);
        }

        if let Some(name) = name {
            if named.insert(name, ancestors).is_some() {
                error!(r#"route names must be unique; duplicate name: "{name}""#);
                #[cfg(debug_assertions)]
                panic!(r#"duplicate route name: "{name}""#)
            }
        }
    }
}

fn match_segment(
    path: &[&str],
    segment: &Segment,
    components: &mut Vec<(Component, BTreeMap<&'static str, Component>)>,
    names: &mut BTreeSet<&'static str>,
    vars: &mut BTreeMap<&'static str, String>,
) -> Option<InternalNavigationTarget> {
    // check static paths
    if let Some((_, route)) = segment.fixed.iter().find(|(p, _)| p == path[0]) {
        match &route.content {
            RouteTarget::TComponent(main, side) => {
                components.push((*main, BTreeMap::from_iter(side.clone().into_iter())));
            }
            RouteTarget::TRedirect(t) => return Some(t.clone()),
        }

        if let Some(name) = &route.name {
            names.insert(name);
        }

        if let Some(sub) = &route.sub {
            if path.len() == 1 {
                if let Some(content) = &sub.index {
                    match content {
                        RouteTarget::TComponent(main, side) => {
                            components.push((*main, BTreeMap::from_iter(side.clone().into_iter())))
                        }
                        RouteTarget::TRedirect(t) => return Some(t.clone()),
                    }
                }
            } else if path.len() > 1 {
                return match_segment(&path[1..], sub, components, names, vars);
            }
        }
    } else {
        match &segment.dynamic {
            DynamicRoute::None => {}
            DynamicRoute::Variable {
                name,
                key,
                content,
                sub,
            } => {
                match content {
                    RouteTarget::TComponent(main, side) => {
                        components.push((*main, BTreeMap::from_iter(side.clone().into_iter())))
                    }
                    RouteTarget::TRedirect(t) => return Some(t.clone()),
                }

                if let Ok(val) = decode(path[0]) {
                    vars.insert(key, val.into_owned());
                }
                if let Some(name) = name {
                    names.insert(name);
                }

                if let Some(sub) = sub.as_deref() {
                    if path.len() == 1 {
                        if let Some(content) = &sub.index {
                            match content {
                                RouteTarget::TComponent(main, side) => components
                                    .push((*main, BTreeMap::from_iter(side.clone().into_iter()))),
                                RouteTarget::TRedirect(t) => return Some(t.clone()),
                            }
                        }
                    } else if path.len() > 1 {
                        return match_segment(&path[1..], sub, components, names, vars);
                    }
                }
            }
            DynamicRoute::Fallback(content) => match content {
                RouteTarget::TComponent(main, side) => {
                    components.push((*main, BTreeMap::from_iter(side.clone().into_iter())))
                }
                RouteTarget::TRedirect(t) => return Some(t.clone()),
            },
        }
    };

    None
}
