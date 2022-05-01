use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock, Weak},
};

use dioxus_core::{Component, ScopeId};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::StreamExt;
use log::error;

use crate::{
    contexts::RouterContext,
    helpers::construct_named_path,
    history::{HistoryProvider, MemoryHistoryProvider},
    prelude::{NamedNavigationSegment, NavigationTarget},
    route_definition::{DynamicRoute, Segment},
    state::CurrentRoute,
};

/// A set of messages that the [`RouterService`] can handle.
pub(crate) enum RouterMessage {
    /// Go back a step in the navigation history.
    GoBack,

    /// Go a step forward in the navigation history.
    GoForward,

    /// Push a new path.
    Push(NavigationTarget),

    /// Replace the current path with a new one.
    Replace(NavigationTarget),

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
                    NavigationTarget::RPath(path) => self.history.push(path),
                    NavigationTarget::RName(name, vars) => {
                        let path = construct_named_path(name, &vars, &self.named_routes)
                            .or(self.named_navigation_fallback_path.clone());
                        if let Some(path) = path {
                            self.history.push(path);
                        }
                    }
                    NavigationTarget::RExternal(url) => {
                        if self.history.can_handle_external() {
                            self.history.push(url);
                        } else {
                            error!("the current history provider cannot handle external navigation targets; pass it to a `Link` component or render an `a` tag instead.");
                        }
                    }
                },
                RouterMessage::Replace(target) => match target {
                    NavigationTarget::RPath(path) => self.history.replace(path),
                    NavigationTarget::RName(name, vars) => {
                        let path = construct_named_path(name, &vars, &self.named_routes)
                            .or(self.named_navigation_fallback_path.clone());
                        if let Some(path) = path {
                            self.history.replace(path);
                        }
                    }
                    NavigationTarget::RExternal(url) => {
                        if self.history.can_handle_external() {
                            self.history.replace(url);
                        } else {
                            error!("the current history provider cannot handle external navigation targets; pass it to a `Link` component or render an `a` tag instead.");
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
        let mut state = self.state.write().unwrap();
        let mut path = self.history.current_path().to_string();

        // clear state
        let CurrentRoute {
            can_go_back,
            can_go_forward,
            components,
            names,
            path: s_path,
            variables,
        } = &mut *state;
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
        if path.len() == 0 {
            if let Some(comp) = self.routes.index {
                components.push(comp);
                names.insert("root_index");
            }
        } else {
            match_segment(&segments, &self.routes, components, names, variables);
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
        component: _,
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
    components: &mut Vec<Component>,
    names: &mut BTreeSet<&'static str>,
    vars: &mut BTreeMap<&'static str, String>,
) {
    // check static paths
    if let Some((_, route)) = segment.fixed.iter().find(|(p, _)| p == path[0]) {
        components.push(route.component);
        if let Some(name) = &route.name {
            names.insert(name);
        }

        if let Some(sub) = &route.sub {
            if path.len() == 1 && sub.index.is_some() {
                components.push(sub.index.unwrap());
            } else if path.len() > 1 {
                match_segment(&path[1..], sub, components, names, vars);
            }
        }
    } else {
        match &segment.dynamic {
            DynamicRoute::None => {}
            DynamicRoute::Variable {
                name,
                key,
                component,
                sub,
            } => {
                components.push(*component);
                vars.insert(key, path[0].to_string());
                if let Some(name) = name {
                    names.insert(name);
                }

                if let Some(sub) = sub.as_deref() {
                    if path.len() == 1 && sub.index.is_some() {
                        components.push(sub.index.unwrap());
                    } else if path.len() > 1 {
                        match_segment(&path[1..], sub, components, names, vars)
                    }
                }
            }
            DynamicRoute::Fallback(comp) => {
                components.push(*comp);
            }
        }
    };
}
