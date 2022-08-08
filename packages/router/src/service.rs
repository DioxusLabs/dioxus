// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol

use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    sync::{Arc, RwLock, Weak},
};

use dioxus::prelude::*;
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::StreamExt;
use log::{error, warn};
use urlencoding::decode;

#[cfg(not(all(feature = "web", target_family = "wasm")))]
use crate::history::MemoryHistory;
#[cfg(all(feature = "web", target_family = "wasm"))]
use crate::history::WebHistory;
use crate::{
    contexts::RouterContext,
    helpers::construct_named_path,
    history::HistoryProvider,
    names::RootIndex,
    navigation::{NamedNavigationSegment, NavigationTarget},
    route_definition::{RouteContent, Segment},
    state::RouterState,
};

/// A set of messages that the [`RouterService`] can handle.
#[derive(Debug)]
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
    fallback_external_navigation: Component,
    fallback_named_navigation: Component,
    history: Box<dyn HistoryProvider>,
    named_routes: Arc<BTreeMap<TypeId, Vec<NamedNavigationSegment>>>,
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
        history: Option<Box<dyn HistoryProvider>>,
        fallback_external_navigation: Component,
        fallback_named_navigation: Component,
    ) -> (Self, RouterContext) {
        // create channel
        let (tx, rx) = unbounded();

        // create named navigation targets
        let mut named_routes = BTreeMap::new();
        construct_named_targets(&routes, &Vec::new(), &mut named_routes);
        named_routes.insert(TypeId::of::<RootIndex>(), Vec::new());
        let named_routes = Arc::new(named_routes);

        // create state and context
        let state = Arc::new(RwLock::new(RouterState::new()));
        let context = RouterContext {
            tx: tx.clone(),
            state: state.clone(),
            named_routes: named_routes.clone(),
        };

        // initiate the history provider
        #[cfg(not(all(feature = "web", target_family = "wasm")))]
        let mut history = history.unwrap_or_else(|| Box::new(MemoryHistory::default()));
        #[cfg(all(feature = "web", target_family = "wasm"))]
        let mut history = history.unwrap_or_else(|| Box::new(WebHistory::default()));
        history.foreign_navigation_handler(Arc::new(move || {
            let _ = tx.unbounded_send(RouterMessage::Update);
        }));

        (
            Self {
                fallback_external_navigation,
                fallback_named_navigation,
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
                    NavigationTarget::InternalTarget(path) => self.history.push(path),
                    NavigationTarget::NamedTarget(name, vars, query) => {
                        match construct_named_path(&name, &vars, &query, &self.named_routes) {
                            Some(path) => self.history.push(path),
                            None => {
                                self.failed_named_navigation();
                                self.update_subscribers();
                                continue; // routing already updated
                            }
                        }
                    }
                    NavigationTarget::ExternalTarget(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                        } else {
                            self.failed_external_navigation(url);
                            self.update_subscribers();
                            continue; // routing already updated
                        }
                    }
                },
                RouterMessage::Replace(target) => match target {
                    NavigationTarget::InternalTarget(path) => self.history.replace(path),
                    NavigationTarget::NamedTarget(name, vars, query) => {
                        match construct_named_path(&name, &vars, &query, &self.named_routes) {
                            Some(path) => self.history.replace(path),
                            None => {
                                self.failed_named_navigation();
                                self.update_subscribers();
                                continue; // routing already updated
                            }
                        }
                    }
                    NavigationTarget::ExternalTarget(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                        } else {
                            self.failed_external_navigation(url);
                            self.update_subscribers();
                            continue; // routing already updated
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
        // prepare variables
        let mut state = self.state.write().unwrap();
        let mut external_navigation_failure = None;
        let mut named_navigation_failure = false;

        loop {
            // clear state
            clear_state(&mut state, &*self.history);
            let RouterState {
                can_external: _,
                can_go_back: _,
                can_go_forward: _,
                components,
                names,
                path,
                prefix: _,
                query: _,
                parameters,
            } = &mut *state;

            // normalize and split path
            let mut path = path.clone();
            path.remove(0);
            let empty_root = path == "/";
            if path.ends_with('/') {
                path.remove(path.len() - 1);
            }
            let segments: Vec<_> = path.split('/').collect();

            // index on root
            let next = if path.is_empty() && !empty_root {
                names.insert(TypeId::of::<RootIndex>());
                self.routes.index.add_to_list(components)
            }
            // all other cases
            else {
                match_segment(
                    &segments,
                    &self.routes,
                    components,
                    names,
                    parameters,
                    &RouteContent::RcNone,
                )
            };

            if let Some(target) = next {
                let target = match target {
                    NavigationTarget::InternalTarget(p) => p,
                    NavigationTarget::NamedTarget(name, vars, query_params) => {
                        match construct_named_path(&name, &vars, &query_params, &self.named_routes)
                        {
                            Some(path) => path,
                            None => {
                                named_navigation_failure = true;
                                break;
                            }
                        }
                    }
                    NavigationTarget::ExternalTarget(url) => {
                        if self.history.can_external() {
                            self.history.external(url);
                            return;
                        } else {
                            external_navigation_failure = Some(url);
                            break;
                        }
                    }
                };

                self.history.replace(target);
            } else {
                return;
            }
        }

        drop(state);
        if let Some(url) = external_navigation_failure {
            self.failed_external_navigation(url);
        }
        if named_navigation_failure {
            self.failed_named_navigation();
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

    /// Go to the state for an external navigation failure.
    fn failed_external_navigation(&mut self, url: String) {
        self.history.push(String::from("/"));

        // clear state
        let mut state = self.state.write().unwrap();
        clear_state(&mut state, &*self.history);

        // show fallback content
        state.components.0.push(self.fallback_external_navigation);
        state.names.insert(TypeId::of::<RootIndex>());
        state.parameters.insert("url", url);
    }

    /// Go to the state for a named navigation failure.
    fn failed_named_navigation(&mut self) {
        self.history.push(String::from("/"));

        // clear state
        let mut state = self.state.write().unwrap();
        clear_state(&mut state, &*self.history);

        // show fallback content
        state.components.0.push(self.fallback_named_navigation);
        state.names.insert(TypeId::of::<RootIndex>());
    }
}

// [`ScopeId`] (in `update`) doesn't implement [`Debug`]
impl Debug for RouterService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterService")
            .field("history", &self.history)
            .field("named_routes", &self.named_routes)
            .field("routes", &self.routes)
            .field("rx", &self.rx)
            .field("state", &self.state)
            .field("subscribers", &self.subscribers)
            .finish_non_exhaustive()
    }
}

/// Clear `state` using values from `history`.
fn clear_state(state: &mut RouterState, history: &dyn HistoryProvider) {
    state.can_external = history.can_external();
    state.can_go_back = history.can_go_back();
    state.can_go_forward = history.can_go_forward();
    state.components.0.clear();
    state.components.1.clear();
    state.names.clear();
    state.path = history.current_path();
    state.prefix = history.current_prefix().to_string();
    state.query = history.current_query();
    state.parameters.clear();
}

/// Traverse the provided `segment` and populate `named` with the named routes.
fn construct_named_targets(
    segment: &Segment,
    ancestors: &[NamedNavigationSegment],
    targets: &mut BTreeMap<TypeId, Vec<NamedNavigationSegment>>,
) {
    let mut add_named_target = |segment: NamedNavigationSegment,
                                name: &Option<(TypeId, &'static str)>,
                                nested: Option<&Segment>| {
        let mut ancestors = Vec::from(ancestors);
        ancestors.push(segment);

        if let Some(nested) = nested {
            construct_named_targets(nested, &ancestors, targets);
        }

        if let Some((id, name)) = name {
            // check for router internal names
            if [TypeId::of::<RootIndex>()].contains(id) {
                error!(r#"route names cannot be defined by dioxus_router, ignore; name: "{name}""#);
                #[cfg(debug_assertions)]
                panic!(r#"route names cannot be defined by dioxus_router; name: "{name}""#);
                #[cfg(not(debug_assertions))]
                return;
            }

            // check if name is already used
            if targets.insert(*id, ancestors).is_some() {
                error!(r#"route names must be unique, later prevails; duplicate name: "{name}""#);
                #[cfg(debug_assertions)]
                panic!(r#"route names must be unique; duplicate name: "{name}""#,);
            }
        }
    };

    for (path, route) in &segment.fixed {
        add_named_target(
            NamedNavigationSegment::Fixed(path.to_string()),
            &route.name,
            route.nested.as_ref(),
        );
    }

    for (_, pr) in &segment.matching {
        add_named_target(
            NamedNavigationSegment::Parameter(pr.key),
            &pr.name,
            pr.nested.as_ref().map(|b| b.as_ref()),
        );
    }

    if let Some(pr) = &segment.parameter {
        add_named_target(
            NamedNavigationSegment::Parameter(pr.key),
            &pr.name,
            pr.nested.as_ref().map(|b| b.as_ref()),
        );
    }

    // add root name
    if ancestors.is_empty() {
        targets.insert(TypeId::of::<RootIndex>(), vec![]);
    }
}

/// Takes in a `segment` and finds the active routes based on the first `path` value.
///
/// Populates `components`, `names` and `vars` with values found while finding all active routes.
#[must_use]
fn match_segment<'a>(
    path: &[&str],
    segment: &'a Segment,
    components: &mut (Vec<Component>, BTreeMap<&'static str, Vec<Component>>),
    names: &mut BTreeSet<TypeId>,
    parameters: &mut BTreeMap<&'static str, String>,
    mut fallback: &'a RouteContent,
) -> Option<NavigationTarget> {
    let decoded_path = decode(path[0])
        .map(|path| path.to_string())
        .unwrap_or_else(|e| {
            // I'm not sure if this case can happen
            let path = path[0];
            warn!(r#"failed to decode path parameter ("{path}"): {e}"#);
            path.to_string()
        });
    let mut found_route = false;

    // routing info
    let content = RouteContent::default();
    let mut content = &content;
    let mut name = None;
    let mut nested = None;
    let mut key = None;

    // extract data
    if let Some(route) = segment.fixed.get(&decoded_path) {
        found_route = true;
        content = &route.content;
        name = route.name;
        nested = route.nested.as_ref();
    } else if let Some((_, route)) = segment
        .matching
        .iter()
        .find(|(regex, _)| regex.is_match(&decoded_path))
    {
        found_route = true;
        content = &route.content;
        key = Some(route.key);
        name = route.name;
        nested = route.nested.as_ref().map(|b| b.as_ref());
    } else if let Some(route) = &segment.parameter {
        found_route = true;
        content = &route.content;
        key = Some(route.key);
        name = route.name;
        nested = route.nested.as_ref().map(|b| b.as_ref());
    }

    // check if fallback is overwritten
    if !segment.fallback.is_rc_none() {
        fallback = &segment.fallback;
    }

    // content and name
    if let Some(target) = content.add_to_list(components) {
        return Some(target);
    }
    if let Some((name, _)) = name {
        names.insert(name);
    }

    // handle parameter
    if let Some(key) = key {
        if parameters.insert(key, decoded_path).is_some() {
            warn!(r#"encountered parameter with same name twice: {key}, later prevails"#);
        }
    }

    if let Some(nested) = nested {
        // index route
        if path.len() == 1 {
            if let Some(target) = nested.index.add_to_list(components) {
                return Some(target);
            }
        }
        // nested routes
        else {
            return match_segment(&path[1..], nested, components, names, parameters, fallback);
        }
    }

    // handle:
    // 1. too specific paths
    // 2. the absence of an active route on the current segment
    if path.len() > 1 || !found_route {
        components.0.clear();
        components.1.clear();
        names.clear();
        parameters.clear();
        return fallback.add_to_list(components);
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::route_definition::{ParameterRoute, Route};
    use regex::Regex;

    use super::*;

    struct Fixed;
    struct FixedEncoded;
    struct Nested;
    struct Nested2;
    struct Match;
    struct Parameter;

    #[test]
    fn named_targets() {
        let mut targets = BTreeMap::new();
        construct_named_targets(&prepare_segment(), &[], &mut targets);

        assert_eq!(targets.len(), 7);
        assert_eq!(targets[&TypeId::of::<Fixed>()].len(), 1);
        assert_eq!(targets[&TypeId::of::<Nested>()].len(), 1);
        assert_eq!(targets[&TypeId::of::<Nested2>()].len(), 2);
        assert_eq!(targets[&TypeId::of::<Match>()].len(), 1);
        assert_eq!(targets[&TypeId::of::<Parameter>()].len(), 1);
        assert!(targets[&TypeId::of::<RootIndex>()].is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"route names must be unique; duplicate name: "dioxus_router::service::tests::Nested2""#]
    fn named_targets_duplicate_panic_in_debug() {
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name(Nested2)),
            &[],
            &mut BTreeMap::new(),
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_targets_duplicate_override_in_release() {
        let mut targets = BTreeMap::new();
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name(Nested2)),
            &[],
            &mut targets,
        );

        assert_eq!(targets[&TypeId::of::<Nested2>()].len(), 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"route names cannot be defined by dioxus_router; name: "dioxus_router::names::RootIndex""#]
    fn named_targets_internal_name_panic_in_debug_root_index() {
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name(RootIndex)),
            &[],
            &mut BTreeMap::new(),
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_targets_internal_name_ignore_in_release() {
        let mut targets = BTreeMap::new();
        construct_named_targets(
            &prepare_segment().fixed(
                "root_index",
                Route::new(RouteContent::RcNone).name(RootIndex),
            ),
            &[],
            &mut targets,
        );

        assert!(targets[&TypeId::of::<RootIndex>()].is_empty());
    }

    #[test]
    fn match_segment_fixed() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["fixed"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Fixed>()));
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_fixed_encoded() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["fixed-%C3%84%C3%96%C3%9C"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<FixedEncoded>()));
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_index() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["nested"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert_eq!(components.0.len(), 2);
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Nested>()));
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_nested() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["nested", "second-layer"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert_eq!(components.0.len(), 3);
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 2);
        assert!(names.contains(&TypeId::of::<Nested>()));
        assert!(names.contains(&TypeId::of::<Nested2>()));
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_matching() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["m1test"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Match>()));
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters["m1-parameter"], "m1test");
    }

    #[test]
    fn match_segment_matching_encoded() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["m1%C3%84%C3%96%C3%9C"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Match>()));
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters["m1-parameter"], "m1ÄÖÜ");
    }

    #[test]
    fn match_segment_parameter() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["test"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        assert!(ret.is_none());
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Parameter>()));
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters["p-parameter"], "test");
    }

    #[test]
    fn match_segment_redirect() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["nested", "redirect"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        let redirect_correct = if let Some(NavigationTarget::InternalTarget(p)) = ret {
            p == "redirect-path"
        } else {
            false
        };
        assert!(redirect_correct);

        // when redirecting, the caller cleans up the values
        assert_eq!(components.0.len(), 1);
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains(&TypeId::of::<Nested>()));
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_fallback() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["nested", "invalid", "another"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        let fallback_correct = match ret {
            Some(NavigationTarget::InternalTarget(p)) => p == "fallback",
            _ => false,
        };
        assert!(fallback_correct);

        // correctly matched values persist
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert!(names.is_empty());
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_fallback_too_specific() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["nested", "empty", "another"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcNone,
        );

        let fallback_correct = match ret {
            Some(NavigationTarget::InternalTarget(p)) => p == "fallback",
            _ => false,
        };
        assert!(fallback_correct);

        // correctly matched values persist
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert!(names.is_empty());
        assert!(parameters.is_empty());
    }

    #[test]
    fn match_segment_global_fallback() {
        let mut components = (Vec::new(), BTreeMap::new());
        let mut names = BTreeSet::new();
        let mut parameters = BTreeMap::new();

        let ret = match_segment(
            &["fixed", "too-specific"],
            &prepare_segment(),
            &mut components,
            &mut names,
            &mut parameters,
            &RouteContent::RcRedirect(NavigationTarget::InternalTarget(String::from("global"))),
        );

        let fallback_correct = if let Some(NavigationTarget::InternalTarget(p)) = ret {
            p == "global"
        } else {
            false
        };
        assert!(fallback_correct);
        assert!(components.0.is_empty());
        assert!(components.1.is_empty());
        assert!(names.is_empty());
        assert!(parameters.is_empty());
    }

    fn prepare_segment() -> Segment {
        Segment::new()
            .fixed("fixed", Route::new(RouteContent::RcNone).name(Fixed))
            .fixed(
                "fixed-ÄÖÜ",
                Route::new(RouteContent::RcNone).name(FixedEncoded),
            )
            .fixed(
                "nested",
                Route::new(TestComponent as Component).name(Nested).nested(
                    Segment::new()
                        .index(TestComponent as Component)
                        .fixed(
                            "second-layer",
                            Route::new(TestComponent as Component)
                                .name(Nested2)
                                .nested(Segment::new().index(TestComponent as Component)),
                        )
                        .fixed("redirect", "redirect-path")
                        .fixed("empty", Route::new(RouteContent::RcNone))
                        .fallback("fallback"),
                ),
            )
            .matching(
                Regex::new("^m1.*$").unwrap(),
                ParameterRoute::new("m1-parameter", RouteContent::RcNone).name(Match),
            )
            .parameter(ParameterRoute::new("p-parameter", RouteContent::RcNone).name(Parameter))
    }

    #[allow(non_snake_case)]
    fn TestComponent(_: Scope) -> Element {
        unimplemented!()
    }
}
