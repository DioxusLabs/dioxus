use std::{
    collections::HashSet,
    sync::{Arc, RwLock, RwLockWriteGuard},
};

use dioxus::prelude::*;

use crate::{
    history::HistoryProvider, navigation::NavigationTarget, routable::Routable,
    router_cfg::RouterConfiguration,
};

/// An error that can occur when navigating.
pub enum NavigationFailure<R: Routable> {
    /// The router failed to navigate to an external URL.
    External(String),
    /// The router failed to navigate to an internal URL.
    Internal(<R as std::str::FromStr>::Err),
}

/// A function the router will call after every routing update.
pub type RoutingCallback<R> = Arc<dyn Fn(RouterContext<R>) -> Option<NavigationTarget<R>>>;

struct MutableRouterState<R>
where
    R: Routable,
{
    /// Whether there is a previous page to navigate back to.
    ///
    /// Even if this is [`true`], there might not be a previous page. However, it is nonetheless
    /// safe to tell the router to go back.
    can_go_back: bool,
    /// Whether there is a future page to navigate forward to.
    ///
    /// Even if this is [`true`], there might not be a future page. However, it is nonetheless safe
    /// to tell the router to go forward.
    can_go_forward: bool,

    /// The current prefix.
    prefix: Option<String>,

    history: Box<dyn HistoryProvider<R>>,
}

/// A collection of router data that manages all routing functionality.
pub struct RouterContext<R>
where
    R: Routable,
{
    state: Arc<RwLock<MutableRouterState<R>>>,

    subscribers: Arc<RwLock<HashSet<ScopeId>>>,
    subscriber_update: Arc<dyn Fn(ScopeId)>,
    routing_callback: Option<RoutingCallback<R>>,

    failure_external_navigation: fn(Scope) -> Element,
    failure_named_navigation: fn(Scope) -> Element,
    failure_redirection_limit: fn(Scope) -> Element,
}

impl<R: Routable> Clone for RouterContext<R> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            subscribers: self.subscribers.clone(),
            subscriber_update: self.subscriber_update.clone(),
            routing_callback: self.routing_callback.clone(),
            failure_external_navigation: self.failure_external_navigation,
            failure_named_navigation: self.failure_named_navigation,
            failure_redirection_limit: self.failure_redirection_limit,
        }
    }
}

impl<R> RouterContext<R>
where
    R: Routable,
{
    pub(crate) fn new(cfg: RouterConfiguration<R>, mark_dirty: Arc<dyn Fn(ScopeId)>) -> Self
    where
        R: Clone,
    {
        let state = Arc::new(RwLock::new(MutableRouterState {
            can_go_back: false,
            can_go_forward: false,
            prefix: Default::default(),
            history: cfg.history,
        }));

        Self {
            state,
            subscribers: Arc::new(RwLock::new(HashSet::new())),
            subscriber_update: mark_dirty,

            routing_callback: cfg.on_update,

            failure_external_navigation: cfg.failure_external_navigation,
            failure_named_navigation: cfg.failure_named_navigation,
            failure_redirection_limit: cfg.failure_redirection_limit,
        }
    }

    /// Check whether there is a previous page to navigate back to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.state.read().unwrap().can_go_back
    }

    /// Check whether there is a future page to navigate forward to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.state.read().unwrap().can_go_forward
    }

    /// Go back to the previous location.
    ///
    /// Will fail silently if there is no previous location to go to.
    pub fn go_back(&self) {
        self.state.write().unwrap().history.go_back();
        self.update_subscribers();
    }

    /// Go back to the next location.
    ///
    /// Will fail silently if there is no next location to go to.
    pub fn go_forward(&self) {
        self.state.write().unwrap().history.go_forward();
        self.update_subscribers();
    }

    /// Push a new location.
    ///
    /// The previous location will be available to go back to.
    pub fn push(&self, target: NavigationTarget<R>) -> Option<NavigationFailure<R>> {
        let mut state = self.state_mut();
        match target {
            NavigationTarget::Internal(p) => state.history.push(p),
            NavigationTarget::External(e) => return self.external(e),
        }

        self.update_subscribers();
        None
    }

    /// Replace the current location.
    ///
    /// The previous location will **not** be available to go back to.
    pub fn replace(&self, target: NavigationTarget<R>) -> Option<NavigationFailure<R>> {
        let mut state = self.state_mut();
        match target {
            NavigationTarget::Internal(p) => state.history.replace(p),
            NavigationTarget::External(e) => return self.external(e),
        }

        self.update_subscribers();
        None
    }

    /// The route that is currently active.
    pub fn current(&self) -> R
    where
        R: Clone,
    {
        self.state.read().unwrap().history.current_route().clone()
    }

    /// The prefix that is currently active.
    pub fn prefix(&self) -> Option<String> {
        self.state.read().unwrap().prefix.clone()
    }

    fn external(&self, external: String) -> Option<NavigationFailure<R>> {
        let mut state = self.state_mut();
        match state.history.external(external.clone()) {
            true => None,
            false => Some(NavigationFailure::External(external)),
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
}

// #[cfg(test)]
// mod tests {
//     //! The tests for [`RouterContext`] test various functions that are not exposed as public.
//     //! However, several of those have an observable effect on the behavior of exposed functions.
//     //!
//     //! The alternative would be to send messages via the services channel and calling one of the
//     //! `run` functions. However, for readability and clarity, it was chosen to directly call the
//     //! private functions.

//     use std::sync::Mutex;

//     use crate::{
//         history::MemoryHistory,
//         routes::{ParameterRoute, Route, RouteContent},
//     };

//     use super::*;

//     fn test_segment() -> Segment<&'static str> {
//         Segment::content(RouteContent::Content(ContentAtom("index")))
//             .fixed(
//                 "fixed",
//                 Route::content(RouteContent::Content(ContentAtom("fixed"))).name::<bool>(),
//             )
//             .fixed(
//                 "redirect",
//                 Route::content(RouteContent::Redirect(NavigationTarget::Internal(
//                     String::from("fixed"),
//                 ))),
//             )
//             .fixed(
//                 "redirection-loop",
//                 Route::content(RouteContent::Redirect(NavigationTarget::Internal(
//                     String::from("/redirection-loop"),
//                 ))),
//             )
//             .fixed(
//                 "%F0%9F%8E%BA",
//                 Route::content(RouteContent::Content(ContentAtom("🎺"))),
//             )
//             .catch_all(ParameterRoute::empty::<bool>())
//     }

//     #[test]
//     fn new_provides_update_to_history() {
//         struct TestHistory {}

//         impl HistoryProvider for TestHistory {
//             fn current_path(&self) -> String {
//                 todo!()
//             }

//             fn current_query(&self) -> Option<String> {
//                 todo!()
//             }

//             fn go_back(&mut self) {
//                 todo!()
//             }

//             fn go_forward(&mut self) {
//                 todo!()
//             }

//             fn push(&mut self, _path: String) {
//                 todo!()
//             }

//             fn replace(&mut self, _path: String) {
//                 todo!()
//             }

//             fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
//                 callback();
//             }
//         }

//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(TestHistory {}),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );

//         assert!(matches!(
//             s.receiver.try_next().unwrap().unwrap(),
//             RouterMessage::Update
//         ));
//     }

//     #[test]
//     fn update_routing() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/fixed?test=value").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         assert_eq!(s.names, s.state.try_read().unwrap().name_map);
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert_eq!(state.query, Some(String::from("test=value")));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//         assert_eq!(s.names, state.name_map);
//     }

//     #[test]
//     fn update_routing_root_index() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("index")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<RootIndex>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/"));
//         assert!(state.query.is_none());
//         assert!(state.prefix.is_none());
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn update_routing_redirect() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/redirect").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     #[should_panic = "reached redirect limit of 25"]
//     #[cfg(debug_assertions)]
//     fn update_routing_redirect_debug() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/redirection-loop").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();
//     }

//     #[test]
//     #[cfg(not(debug_assertions))]
//     fn update_routing_redirect_release() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/redirection-loop").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("redirect limit")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<FailureRedirectionLimit>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/redirection-loop"));
//         assert_eq!(state.can_go_back, false);
//         assert_eq!(state.can_go_forward, false);
//     }

//     #[test]
//     fn update_subscribers() {
//         let ids = Arc::new(Mutex::new(Vec::new()));
//         let ids2 = Arc::clone(&ids);

//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             Segment::empty(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(move |id| {
//                 ids2.lock().unwrap().push(id);
//             }),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );

//         let id0 = Arc::new(0);
//         s.subscribe(Arc::clone(&id0));

//         let id1 = Arc::new(1);
//         s.subscribe(Arc::clone(&id1));

//         let id1 = Arc::try_unwrap(id1).unwrap();
//         s.update_subscribers();

//         assert_eq!(s.subscribers.len(), 1);
//         assert_eq!(s.subscribers[0].upgrade().unwrap(), id0);
//         assert_eq!(*ids.lock().unwrap(), vec![*id0, id1, *id0]);
//     }

//     #[test]
//     fn push_internal() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.push(NavigationTarget::Internal(String::from("/fixed")));
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn push_named() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.push(NavigationTarget::named::<bool>());
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn push_external() {
//         let (mut s, tx, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();
//         tx.unbounded_send(RouterMessage::Push(NavigationTarget::External(
//             String::from("https://dioxuslabs.com/"),
//         )))
//         .unwrap();
//         s.run_current();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("external target")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<FailureExternalNavigation>());
//             r
//         });
//         assert_eq!(state.parameters, {
//             let mut r = HashMap::new();
//             r.insert(
//                 Name::of::<FailureExternalNavigation>(),
//                 String::from("https://dioxuslabs.com/"),
//             );
//             r
//         });
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn replace_named() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.replace(NavigationTarget::named::<bool>());
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn replace_internal() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.replace(NavigationTarget::Internal(String::from("/fixed")));
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("fixed")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<bool>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn replace_external() {
//         let (mut s, tx, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();
//         tx.unbounded_send(RouterMessage::Replace(NavigationTarget::External(
//             String::from("https://dioxuslabs.com/"),
//         )))
//         .unwrap();
//         s.run_current();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("external target")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<FailureExternalNavigation>());
//             r
//         });
//         assert_eq!(state.parameters, {
//             let mut r = HashMap::new();
//             r.insert(
//                 Name::of::<FailureExternalNavigation>(),
//                 String::from("https://dioxuslabs.com/"),
//             );
//             r
//         });
//         assert_eq!(state.path, String::from("/fixed"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn subscribe() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             Segment::empty(),
//             Box::<MemoryHistory>::default(),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );

//         let id = Arc::new(0);
//         s.subscribe(Arc::clone(&id));

//         assert_eq!(s.subscribers.len(), 1);
//         assert_eq!(s.subscribers[0].upgrade().unwrap(), id);
//     }

//     #[test]
//     fn routing_callback() {
//         let paths = Arc::new(Mutex::new(Vec::new()));
//         let paths2 = Arc::clone(&paths);

//         let (mut s, c, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
//             Arc::new(|_| {}),
//             Some(Arc::new(move |state| {
//                 paths2.lock().unwrap().push(state.path.clone());
//                 Some("/%F0%9F%8E%BA".into())
//             })),
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );

//         assert!(paths.lock().unwrap().is_empty());

//         s.init();
//         assert_eq!(*paths.lock().unwrap(), vec![String::from("/fixed")]);

//         c.unbounded_send(RouterMessage::Update).unwrap();
//         s.run_current();
//         assert_eq!(
//             *paths.lock().unwrap(),
//             vec![String::from("/fixed"), String::from("/%F0%9F%8E%BA")]
//         );

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("🎺")])
//     }

//     #[test]
//     fn url_decoding_do() {
//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/%F0%9F%A5%B3").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert!(state.content.is_empty());
//         assert!(state.names.is_empty());
//         assert_eq!(state.parameters, {
//             let mut r = HashMap::new();
//             r.insert(Name::of::<bool>(), String::from("🥳"));
//             r
//         });
//         assert_eq!(state.path, String::from("/%F0%9F%A5%B3"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn url_decoding_do_not() {
//         let (mut s, c, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(MemoryHistory::with_initial_path("/%F0%9F%8E%BA").unwrap()),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();
//         c.unbounded_send(RouterMessage::Update).unwrap();
//         s.run_current();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("🎺")]);
//         assert!(state.names.is_empty());
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/%F0%9F%8E%BA"));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }

//     #[test]
//     fn prefix() {
//         struct TestHistory {}

//         impl HistoryProvider for TestHistory {
//             fn current_path(&self) -> String {
//                 String::from("/")
//             }

//             fn current_query(&self) -> Option<String> {
//                 None
//             }

//             fn current_prefix(&self) -> Option<String> {
//                 Some(String::from("/prefix"))
//             }

//             fn can_go_back(&self) -> bool {
//                 false
//             }

//             fn can_go_forward(&self) -> bool {
//                 false
//             }

//             fn go_back(&mut self) {
//                 todo!()
//             }

//             fn go_forward(&mut self) {
//                 todo!()
//             }

//             fn push(&mut self, _path: String) {
//                 todo!()
//             }

//             fn replace(&mut self, _path: String) {
//                 todo!()
//             }

//             fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
//                 callback();
//             }
//         }

//         let (mut s, _, _) = RouterContext::<_, u8>::new(
//             test_segment(),
//             Box::new(TestHistory {}),
//             Arc::new(|_| {}),
//             None,
//             ContentAtom("external target"),
//             ContentAtom("named target"),
//             ContentAtom("redirect limit"),
//         );
//         s.init();

//         let state = s.state.try_read().unwrap();
//         assert_eq!(state.content, vec![ContentAtom("index")]);
//         assert_eq!(state.names, {
//             let mut r = HashSet::new();
//             r.insert(Name::of::<RootIndex>());
//             r
//         });
//         assert!(state.parameters.is_empty());
//         assert_eq!(state.path, String::from("/"));
//         assert!(state.query.is_none());
//         assert_eq!(state.prefix, Some(String::from("/prefix")));
//         assert!(!state.can_go_back);
//         assert!(!state.can_go_forward);
//     }
// }
