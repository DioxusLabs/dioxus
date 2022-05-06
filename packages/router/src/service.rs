use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock, Weak},
};

use dioxus_core::{Component, ScopeId};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::StreamExt;
use log::error;
use urlencoding::{decode, encode};

#[cfg(feature = "web")]
use crate::history::BrowserPathHistoryProvider;
#[cfg(not(feature = "web"))]
use crate::history::MemoryHistoryProvider;
use crate::{
    contexts::RouterContext,
    helpers::construct_named_path,
    history::HistoryProvider,
    navigation::{NamedNavigationSegment, NavigationTarget},
    route_definition::{DynamicRoute, RouteContent, Segment},
    state::RouterState,
    PATH_FOR_EXTERNAL_NAVIGATION_FAILURE, PATH_FOR_NAMED_NAVIGATION_FAILURE,
};

/// A set of messages that the [`RouterService`] can handle.
pub(crate) enum RouterMessage {
    /// Go back a step in the navigation history.
    GoBack,

    /// Go a step forward in the navigation history.
    GoForward,

    /// Push a new history item.
    Push(NavigationTarget),

    /// Replace the current history item with a new one.
    Replace(NavigationTarget),

    /// Subscribe the specified scope to router updates.
    Subscribe(Arc<ScopeId>),

    /// Tell the router to update the current state.
    Update,
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
    global_fallback: RouteContent,
    history: Box<dyn HistoryProvider>,
    named_routes: Arc<BTreeMap<&'static str, Vec<NamedNavigationSegment>>>,
    routes: Arc<Segment>,
    rx: UnboundedReceiver<RouterMessage>,
    state: Arc<RwLock<RouterState>>,
    subscribers: Vec<Weak<ScopeId>>,
    update: Arc<dyn Fn(ScopeId)>,
}

impl RouterService {
    /// Create a new [`RouterService`].
    ///
    /// The returned [`RouterService`] and [`RouterContext`] are linked with each other.
    #[must_use]
    pub(crate) fn new(
        routes: Arc<Segment>,
        update: Arc<dyn Fn(ScopeId)>,
        active_class: Option<String>,
        global_fallback: RouteContent,
        history: Option<Box<dyn HistoryProvider>>,
    ) -> (Self, RouterContext) {
        // create channel
        let (tx, rx) = unbounded();

        // create named navigation targets
        let mut named_routes = BTreeMap::new();
        construct_named_targets(&routes, &Vec::new(), &mut named_routes);
        named_routes.insert("root_index", Vec::new());
        let named_routes = Arc::new(named_routes);

        // create state and context
        let state = Arc::new(RwLock::new(RouterState::default()));
        let context = RouterContext {
            active_class,
            tx: tx.clone(),
            state: state.clone(),
            named_routes: named_routes.clone(),
        };

        // initiate the history provider
        #[cfg(not(feature = "web"))]
        let mut history = history.unwrap_or(Box::new(MemoryHistoryProvider::default()));
        #[cfg(feature = "web")]
        let mut history = history.unwrap_or(Box::new(BrowserPathHistoryProvider::default()));
        history.foreign_navigation_handler(Arc::new(move || {
            tx.unbounded_send(RouterMessage::Update).ok();
        }));

        (
            Self {
                global_fallback,
                history,
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

    /// Perform a single routing operation. Doesn't trigger updates.
    pub(crate) fn single_routing(&mut self) {
        self.update_routing();
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
                    NavigationTarget::NtPath(path) => self.history.push(path),
                    NavigationTarget::NtName(name, vars, query) => self.history.push(
                        construct_named_path(name, &vars, &query, &self.named_routes)
                            .unwrap_or(PATH_FOR_NAMED_NAVIGATION_FAILURE.to_string()),
                    ),
                    NavigationTarget::NtExternal(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                        } else {
                            self.history.push(format!(
                                "/{path}?url={url}",
                                path = PATH_FOR_EXTERNAL_NAVIGATION_FAILURE,
                                url = encode(&url)
                            ));
                        }
                    }
                },
                RouterMessage::Replace(target) => match target {
                    NavigationTarget::NtPath(path) => self.history.replace(path),
                    NavigationTarget::NtName(name, vars, query) => self.history.replace(
                        construct_named_path(name, &vars, &query, &self.named_routes)
                            .unwrap_or(PATH_FOR_NAMED_NAVIGATION_FAILURE.to_string()),
                    ),
                    NavigationTarget::NtExternal(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                        } else {
                            self.history.replace(format!(
                                "/{path}?url={url}",
                                path = PATH_FOR_EXTERNAL_NAVIGATION_FAILURE,
                                url = encode(&url)
                            ));
                        }
                    }
                },
                RouterMessage::Subscribe(id) => {
                    self.subscribers.push(Arc::downgrade(&id));
                    (self.update)(*id);
                    continue; // no navigation happened
                }
                RouterMessage::Update => { /* update triggered at end of loop */ }
            }

            self.update_routing();
            self.update_subscribers();
        }
    }

    /// Update the current state of the router.
    fn update_routing(&mut self) {
        // prepare varibles
        let mut state = self.state.write().unwrap();
        let RouterState {
            can_external,
            can_go_back,
            can_go_forward,
            components,
            names,
            path,
            prefix,
            query,
            parameters: variables,
        } = &mut *state;

        loop {
            // clear state
            *can_external = self.history.can_external();
            *can_go_back = self.history.can_go_back();
            *can_go_forward = self.history.can_go_forward();
            components.0.clear();
            components.1.clear();
            names.clear();
            *path = self.history.current_path();
            *prefix = self.history.current_prefix().to_string();
            *query = self.history.current_query();
            variables.clear();

            // normalize and split path
            let mut path = path.clone();
            path.remove(0);
            if path.ends_with("/") {
                path.remove(path.len() - 1);
            }
            let segments: Vec<_> = path.split("/").collect();

            // handle index on root
            let next = if path.len() == 0 {
                names.insert("root_index");
                self.routes.index.add_to_list(components)
            } else {
                match_segment(
                    &segments,
                    &self.routes,
                    components,
                    names,
                    variables,
                    &self.global_fallback,
                )
            };

            if let Some(target) = next {
                self.history.replace(match target {
                    NavigationTarget::NtPath(p) => p,
                    NavigationTarget::NtName(name, vars, query_params) => {
                        construct_named_path(name, &vars, &query_params, &self.named_routes)
                            .unwrap_or(PATH_FOR_NAMED_NAVIGATION_FAILURE.to_string())
                    }
                    NavigationTarget::NtExternal(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                            break;
                        } else {
                            format!(
                                "/{path}?url={url}",
                                path = PATH_FOR_EXTERNAL_NAVIGATION_FAILURE,
                                url = encode(&url)
                            )
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

/// Traverse the provided `segment` and populate `named` with the named routes.
fn construct_named_targets(
    segment: &Segment,
    ancestors: &Vec<NamedNavigationSegment>,
    named: &mut BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) {
    for (path, route) in &segment.fixed {
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

    if let DynamicRoute::Parameter {
        name,
        key,
        content: _,
        sub,
    } = &segment.dynamic
    {
        let mut ancestors = ancestors.clone();
        ancestors.push(NamedNavigationSegment::Parameter(key));

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

/// Match the first `path` segment with the `segment`.
///
/// Populates `components`, `names` and `vars` with the values found while matching.
///
/// If `path` contains more segments than specified by `segment`, `names` and `vars` will be empty
/// and `components` will only contain the `global_fallback`. This only applies if `components` is
/// not [`RouteContent::RcNone`].
#[must_use]
fn match_segment(
    path: &[&str],
    segment: &Segment,
    components: &mut (Vec<Component>, BTreeMap<&'static str, Vec<Component>>),
    names: &mut BTreeSet<&'static str>,
    vars: &mut BTreeMap<&'static str, String>,
    global_fallback: &RouteContent,
) -> Option<NavigationTarget> {
    // check static paths
    if let Some(route) = segment.fixed.get(path[0]) {
        if let Some(name) = &route.name {
            names.insert(name);
        }

        if let Some(target) = route.content.add_to_list(components) {
            return Some(target);
        }

        if let Some(sub) = &route.sub {
            if path.len() == 1 {
                if let Some(target) = sub.index.add_to_list(components) {
                    return Some(target);
                }
            } else if path.len() > 1 {
                return match_segment(&path[1..], sub, components, names, vars, global_fallback);
            }
        } else if path.len() > 1 && !global_fallback.is_rc_none() {
            components.0.clear();
            components.1.clear();
            names.clear();
            vars.clear();
            return global_fallback.add_to_list(components);
        }
    } else {
        match &segment.dynamic {
            DynamicRoute::None => {
                if !global_fallback.is_rc_none() {
                    components.0.clear();
                    components.1.clear();
                    names.clear();
                    vars.clear();
                    return global_fallback.add_to_list(components);
                }
            }
            DynamicRoute::Parameter {
                name,
                key,
                content,
                sub,
            } => {
                if let Some(name) = name {
                    names.insert(name);
                }

                if let Some(target) = content.add_to_list(components) {
                    return Some(target);
                }

                if let Ok(val) = decode(path[0]) {
                    vars.insert(key, val.into_owned());
                }

                if let Some(sub) = sub.as_deref() {
                    if path.len() == 1 {
                        if let Some(target) = sub.index.add_to_list(components) {
                            return Some(target);
                        }
                    } else if path.len() > 1 {
                        return match_segment(
                            &path[1..],
                            sub,
                            components,
                            names,
                            vars,
                            global_fallback,
                        );
                    }
                } else if path.len() > 1 && !global_fallback.is_rc_none() {
                    components.0.clear();
                    components.1.clear();
                    names.clear();
                    vars.clear();
                    return global_fallback.add_to_list(components);
                }
            }
            DynamicRoute::Fallback(content) => {
                if path.len() > 1 && !global_fallback.is_rc_none() {
                    components.0.clear();
                    components.1.clear();
                    names.clear();
                    vars.clear();
                    return global_fallback.add_to_list(components);
                } else {
                    return content.add_to_list(components);
                }
            }
        }
    };

    None
}
