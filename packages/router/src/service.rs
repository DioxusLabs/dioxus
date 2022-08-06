// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    sync::{Arc, RwLock, Weak},
};

use dioxus::prelude::*;
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use futures_util::StreamExt;
use log::{error, warn};
use urlencoding::{decode, encode};

#[cfg(not(all(feature = "web", target_family = "wasm")))]
use crate::history::MemoryHistory;
#[cfg(all(feature = "web", target_family = "wasm"))]
use crate::history::WebHistory;
use crate::{
    contexts::RouterContext,
    helpers::construct_named_path,
    history::HistoryProvider,
    navigation::{NamedNavigationSegment, NavigationTarget},
    route_definition::{RouteContent, Segment},
    state::RouterState,
    PATH_FOR_EXTERNAL_NAVIGATION_FAILURE, PATH_FOR_NAMED_NAVIGATION_FAILURE,
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
        history: Option<Box<dyn HistoryProvider>>,
    ) -> (Self, RouterContext) {
        // create channel
        let (tx, rx) = unbounded();

        // create named navigation targets
        let mut named_routes = BTreeMap::new();
        construct_named_targets(&routes, &Vec::new(), &mut named_routes);
        named_routes.insert("", Vec::new());
        let named_routes = Arc::new(named_routes);

        // create state and context
        let state = Arc::new(RwLock::new(RouterState::new()));
        let context = RouterContext {
            active_class,
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
                            .unwrap_or(format!("/{PATH_FOR_NAMED_NAVIGATION_FAILURE}")),
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
                            .unwrap_or(format!("/{PATH_FOR_NAMED_NAVIGATION_FAILURE}")),
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
        // prepare variables
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
            parameters,
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
            parameters.clear();

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
                names.insert("");
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
                self.history.replace(match target {
                    NavigationTarget::NtPath(p) => p,
                    NavigationTarget::NtName(name, vars, query_params) => {
                        construct_named_path(name, &vars, &query_params, &self.named_routes)
                            .unwrap_or(format!("/{PATH_FOR_NAMED_NAVIGATION_FAILURE}"))
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

/// Traverse the provided `segment` and populate `named` with the named routes.
fn construct_named_targets(
    segment: &Segment,
    ancestors: &[NamedNavigationSegment],
    targets: &mut BTreeMap<&'static str, Vec<NamedNavigationSegment>>,
) {
    let mut add_named_target = |segment: NamedNavigationSegment,
                                name: &Option<&'static str>,
                                nested: Option<&Segment>| {
        let mut ancestors = Vec::from(ancestors);
        ancestors.push(segment);

        if let Some(nested) = nested {
            construct_named_targets(nested, &ancestors, targets);
        }

        if let Some(name) = name {
            if targets.insert(name, ancestors).is_some() {
                error!(r#"route names must be unique, later prevails; duplicate name: "{name}""#);
                #[cfg(debug_assertions)]
                panic!(r#"route names must be unique; duplicate name: "{name}""#);
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
    if ancestors.is_empty() && targets.insert("", vec![]).is_some() {
        error!(r#"root route name ("" -> "/") is provided by router, custom is overwritten"#);
        #[cfg(debug_assertions)]
        panic!(r#"root route name ("" -> "/") is provided by router"#);
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
    names: &mut BTreeSet<&'static str>,
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
    if let Some(name) = name {
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

    #[test]
    fn named_targets() {
        let mut targets = BTreeMap::new();
        construct_named_targets(&prepare_segment(), &[], &mut targets);

        assert_eq!(targets.len(), 7);
        assert_eq!(targets["fixed"].len(), 1);
        assert_eq!(targets["nested"].len(), 1);
        assert_eq!(targets["nested2"].len(), 2);
        assert_eq!(targets["match"].len(), 1);
        assert_eq!(targets["parameter"].len(), 1);
        assert!(targets[""].is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"route names must be unique; duplicate name: "nested2""#]
    fn named_targets_duplicate_panic_in_debug() {
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name("nested2")),
            &[],
            &mut BTreeMap::new(),
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_targets_duplicate_override_in_release() {
        let mut targets = BTreeMap::new();
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name("nested2")),
            &[],
            &mut targets,
        );

        assert_eq!(targets["nested2"].len(), 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = r#"root route name ("" -> "/") is provided by router"#]
    fn named_targets_root_panic_in_debug() {
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name("")),
            &[],
            &mut BTreeMap::new(),
        );
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn named_targets_root_override_in_release() {
        let mut targets = BTreeMap::new();
        construct_named_targets(
            &prepare_segment().fixed("test", Route::new(RouteContent::RcNone).name("")),
            &[],
            &mut targets,
        );

        assert!(targets[""].is_empty());
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
        assert!(names.contains("fixed"));
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
        assert!(names.contains("fixed-encoded"));
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
        assert!(names.contains("nested"));
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
        assert!(names.contains("nested"));
        assert!(names.contains("nested2"));
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
        assert!(names.contains("match"));
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
        assert!(names.contains("match"));
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
        assert!(names.contains("parameter"));
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

        let redirect_correct = if let Some(NavigationTarget::NtPath(p)) = ret {
            p == "redirect-path"
        } else {
            false
        };
        assert!(redirect_correct);

        // when redirecting, the caller cleans up the values
        assert_eq!(components.0.len(), 1);
        assert!(components.1.is_empty());
        assert_eq!(names.len(), 1);
        assert!(names.contains("nested"));
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
            Some(NavigationTarget::NtPath(p)) => p == "fallback",
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
            Some(NavigationTarget::NtPath(p)) => p == "fallback",
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
            &RouteContent::RcRedirect(NavigationTarget::NtPath(String::from("global"))),
        );

        let fallback_correct = if let Some(NavigationTarget::NtPath(p)) = ret {
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
            .fixed("fixed", Route::new(RouteContent::RcNone).name("fixed"))
            .fixed(
                "fixed-ÄÖÜ",
                Route::new(RouteContent::RcNone).name("fixed-encoded"),
            )
            .fixed(
                "nested",
                Route::new(RouteContent::RcComponent(TestComponent))
                    .name("nested")
                    .nested(
                        Segment::new()
                            .index(RouteContent::RcComponent(TestComponent))
                            .fixed(
                                "second-layer",
                                Route::new(RouteContent::RcComponent(TestComponent))
                                    .name("nested2")
                                    .nested(
                                        Segment::new()
                                            .index(RouteContent::RcComponent(TestComponent)),
                                    ),
                            )
                            .fixed(
                                "redirect",
                                Route::new(RouteContent::RcRedirect(NavigationTarget::NtPath(
                                    String::from("redirect-path"),
                                ))),
                            )
                            .fixed("empty", Route::new(RouteContent::RcNone))
                            .fallback(RouteContent::RcRedirect(NavigationTarget::NtPath(
                                String::from("fallback"),
                            ))),
                    ),
            )
            .matching(
                Regex::new("^m1.*$").unwrap(),
                ParameterRoute::new("m1-parameter", RouteContent::RcNone).name("match"),
            )
            .parameter(ParameterRoute::new("p-parameter", RouteContent::RcNone).name("parameter"))
    }

    #[allow(non_snake_case)]
    fn TestComponent(_: Scope) -> Element {
        unimplemented!()
    }
}
