use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    sync::{Arc, Weak},
};

use async_lock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use either::Either;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;

use crate::{
    history::HistoryProvider,
    navigation::NavigationTarget,
    prelude::{
        FailureExternalNavigation, FailureNamedNavigation, FailureRedirectionLimit, RootIndex,
    },
    routes::{ContentAtom, Segment},
    segments::{NameMap, NamedSegment},
    utils::{resolve_target, route_segment},
    Name, RouterState,
};

/// Messages that the [`RouterService`] can handle.
pub enum RouterMessage<I> {
    /// Subscribe to router update.
    Subscribe(Arc<I>),
    /// Navigate to the specified target.
    Push(NavigationTarget),
    /// Replace the current location with the specified target.
    Replace(NavigationTarget),
    /// Trigger a routing update.
    Update,
    /// Navigate to the previous history entry.
    GoBack,
    /// Navigate to the next history entry.
    GoForward,
}

impl<I: Debug> Debug for RouterMessage<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subscribe(arg0) => f.debug_tuple("Subscribe").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Replace(arg0) => f.debug_tuple("Replace").field(arg0).finish(),
            Self::Update => write!(f, "Update"),
            Self::GoBack => write!(f, "GoBack"),
            Self::GoForward => write!(f, "GoForward"),
        }
    }
}

impl<I: PartialEq> PartialEq for RouterMessage<I> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Subscribe(l0), Self::Subscribe(r0)) => l0 == r0,
            (Self::Push(l0), Self::Push(r0)) => l0 == r0,
            (Self::Replace(l0), Self::Replace(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl<I: Eq> Eq for RouterMessage<I> {}

enum NavigationFailure {
    External(String),
    Named(Name),
}

/// A function the router will call after every routing update.
pub type RoutingCallback<T> = Arc<dyn Fn(&RouterState<T>) -> Option<NavigationTarget>>;

/// A collection of router data that manages all routing functionality.
pub struct RouterService<T, I>
where
    T: Clone,
    I: Clone + PartialEq + Eq,
{
    history: Box<dyn HistoryProvider>,
    routes: Segment<T>,
    names: Arc<NameMap>,

    receiver: UnboundedReceiver<RouterMessage<I>>,
    state: Arc<RwLock<RouterState<T>>>,

    subscribers: Vec<Weak<I>>,
    subscriber_update: Arc<dyn Fn(I)>,
    routing_callback: Option<RoutingCallback<T>>,

    failure_external_navigation: ContentAtom<T>,
    failure_named_navigation: ContentAtom<T>,
    failure_redirection_limit: ContentAtom<T>,
}

impl<T, I> RouterService<T, I>
where
    T: Clone,
    I: Clone + PartialEq + Eq + Send + Sync + 'static,
{
    /// Create a new [`RouterService`].
    ///
    /// # Parameters
    /// 1. `routes`: The root [`Segment`] the router should handle.
    /// 2. `history`: A [`HistoryProvider`] to handle the navigation history.
    /// 3. `subscriber_callback`: A function the rooter can call to update UI integrations.
    /// 4. `failure_external_navigation`: Content to be displayed when an external navigation fails.
    /// 5. `failure_named_navigation`: Content to be displayed when a named navigation fails.
    /// 6. `failure_redirection_limit`: Content to be displayed when the redirection limit is
    ///    breached.
    ///
    /// # Returns
    /// 1. The [`RouterService`].
    /// 2. An [`UnboundedSender`] to send [`RouterMessage`]s to the [`RouterService`].
    /// 3. Access to the [`RouterState`]. **DO NOT WRITE TO THIS!!!** Seriously, **READ ONLY!!!**
    #[allow(clippy::type_complexity)]
    pub fn new(
        routes: Segment<T>,
        mut history: Box<dyn HistoryProvider>,
        subscriber_update: Arc<dyn Fn(I)>,
        routing_callback: Option<RoutingCallback<T>>,
        failure_external_navigation: ContentAtom<T>,
        failure_named_navigation: ContentAtom<T>,
        failure_redirection_limit: ContentAtom<T>,
    ) -> (
        Self,
        UnboundedSender<RouterMessage<I>>,
        Arc<RwLock<RouterState<T>>>,
    ) {
        // index names
        let names = Arc::new(NamedSegment::from_segment(&routes));

        // create channel
        let (sender, receiver) = unbounded();

        // initialize history
        let history_sender = sender.clone();
        history.updater(Arc::new(move || {
            let _ = history_sender.unbounded_send(RouterMessage::Update);
        }));
        let state = Arc::new(RwLock::new(RouterState {
            name_map: Arc::clone(&names),
            ..Default::default()
        }));

        (
            Self {
                history,
                names,
                routes,
                receiver,
                state: Arc::clone(&state),
                subscribers: Vec::new(),
                subscriber_update,
                routing_callback,
                failure_external_navigation,
                failure_named_navigation,
                failure_redirection_limit,
            },
            sender,
            state,
        )
    }

    /// Perform the initial routing.
    ///
    /// Call this once, as soon as possible after creating the [`RouterService`]. Do not call this,
    /// if you are going to call the `run` function.
    pub fn init(&mut self) {
        *self.sync_state_write_lock() = self
            .update_routing()
            .left_and_then(|state| {
                if let Some(cb) = &self.routing_callback {
                    if let Some(nt) = cb(&state) {
                        self.replace(nt);
                        return self.update_routing();
                    }
                }

                Either::Left(state)
            })
            .map_right(|err| self.handle_navigation_failure(&self.sync_state_read_lock(), err))
            .either_into();
    }

    /// Handle all messages the router has received and then return.
    ///
    /// Call `init` before calling this function.
    pub fn run_current(&mut self) {
        let mut state = None;
        while let Ok(Some(msg)) = self.receiver.try_next() {
            let current = match self.handle_message(msg) {
                (_, Some(err)) => Either::Right(err),
                (true, _) => self.update_routing(),
                _ => continue,
            }
            .left_and_then(|state| {
                if let Some(cb) = &self.routing_callback {
                    if let Some(nt) = cb(&state) {
                        self.replace(nt);
                        return self.update_routing();
                    }
                }

                Either::Left(state)
            })
            .map_right(|err| self.handle_navigation_failure(&self.sync_state_read_lock(), err))
            .either_into();
            state = Some(current);
        }

        if let Some(state) = state {
            *self.sync_state_write_lock() = state;
        }

        self.update_subscribers();
    }

    fn sync_state_read_lock(&self) -> RwLockReadGuard<RouterState<T>> {
        loop {
            if let Some(s) = self.state.try_read() {
                return s;
            }
        }
    }

    fn sync_state_write_lock(&mut self) -> RwLockWriteGuard<RouterState<T>> {
        loop {
            if let Some(s) = self.state.try_write() {
                return s;
            }
        }
    }

    /// Handle all routing messages until ended from the outside.
    pub async fn run(&mut self) {
        // init (unlike function with same name this is async)
        {
            *self.state.write().await = match self.update_routing().left_and_then(|state| {
                if let Some(cb) = &self.routing_callback {
                    if let Some(nt) = cb(&state) {
                        self.replace(nt);
                        return self.update_routing();
                    }
                }

                Either::Left(state)
            }) {
                Either::Left(state) => state,
                Either::Right(err) => {
                    self.handle_navigation_failure(&*self.state.read().await, err)
                }
            };
        }
        self.update_subscribers();

        while let Some(msg) = self.receiver.next().await {
            let state = match self.handle_message(msg) {
                (_, Some(err)) => Either::Right(err),
                (true, _) => self.update_routing(),
                _ => continue,
            }
            .left_and_then(|state| {
                if let Some(cb) = &self.routing_callback {
                    if let Some(nt) = cb(&state) {
                        self.replace(nt);
                        return self.update_routing();
                    }
                }

                Either::Left(state)
            });

            *self.state.write().await = match state {
                Either::Left(state) => state,
                Either::Right(err) => {
                    self.handle_navigation_failure(&*self.state.read().await, err)
                }
            };

            self.update_subscribers();
        }
    }

    fn handle_navigation_failure(
        &self,
        state: &RouterState<T>,
        err: NavigationFailure,
    ) -> RouterState<T> {
        match err {
            NavigationFailure::External(url) => RouterState {
                can_go_back: state.can_go_back,
                can_go_forward: state.can_go_forward,
                path: state.path.clone(),
                query: state.query.clone(),
                prefix: state.prefix.clone(),
                names: {
                    let mut r = HashSet::new();
                    r.insert(Name::of::<FailureExternalNavigation>());
                    r
                },
                parameters: {
                    let mut r = HashMap::new();
                    r.insert(Name::of::<FailureExternalNavigation>(), url);
                    r
                },
                name_map: Arc::clone(&state.name_map),
                content: vec![self.failure_external_navigation.clone()],
                named_content: BTreeMap::new(),
            },
            NavigationFailure::Named(n) => RouterState {
                can_go_back: state.can_go_back,
                can_go_forward: state.can_go_forward,
                path: state.path.clone(),
                query: state.query.clone(),
                prefix: state.prefix.clone(),
                names: {
                    let mut r = HashSet::new();
                    r.insert(Name::of::<FailureNamedNavigation>());
                    r
                },
                parameters: {
                    let mut r = HashMap::new();
                    r.insert(Name::of::<FailureExternalNavigation>(), n.to_string());
                    r
                },
                name_map: Arc::clone(&state.name_map),
                content: vec![self.failure_named_navigation.clone()],
                named_content: BTreeMap::new(),
            },
        }
    }

    #[must_use]
    fn handle_message(&mut self, msg: RouterMessage<I>) -> (bool, Option<NavigationFailure>) {
        let failure = match msg {
            RouterMessage::Subscribe(id) => {
                self.subscribe(id);
                return (false, None);
            }
            RouterMessage::Push(nt) => self.push(nt),
            RouterMessage::Replace(nt) => self.replace(nt),
            RouterMessage::Update => None,
            RouterMessage::GoBack => {
                self.history.go_back();
                None
            }
            RouterMessage::GoForward => {
                self.history.go_forward();
                None
            }
        };

        (true, failure)
    }

    #[must_use]
    fn update_routing(&mut self) -> Either<RouterState<T>, NavigationFailure> {
        for _ in 0..=25 {
            match self.update_routing_inner() {
                Either::Left(state) => return Either::Left(state),
                Either::Right(nt) => {
                    if let Some(err) = self.replace(nt) {
                        return Either::Right(err);
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        panic!("reached redirect limit of 25");
        #[allow(unreachable_code)]
        Either::Left(RouterState {
            content: vec![self.failure_redirection_limit.clone()],
            can_go_back: self.history.can_go_back(),
            can_go_forward: self.history.can_go_forward(),
            path: self.history.current_path(),
            query: self.history.current_query(),
            prefix: self.history.current_prefix(),
            name_map: Arc::clone(&self.names),
            names: {
                let mut r = HashSet::new();
                r.insert(Name::of::<FailureRedirectionLimit>());
                r
            },
            ..Default::default()
        })
    }

    #[must_use]
    fn update_routing_inner(&mut self) -> Either<RouterState<T>, NavigationTarget> {
        // prepare path
        let mut path = self.history.current_path();
        path.remove(0);
        if path.ends_with('/') {
            path.pop();
        }

        let values = match path.is_empty() {
            false => path.split('/').collect::<Vec<_>>(),
            true => Vec::new(),
        };

        // add root index name
        let mut names = HashSet::new();
        if values.is_empty() {
            names.insert(Name::of::<RootIndex>());
        };

        route_segment(
            &self.routes,
            &values,
            RouterState {
                can_go_back: self.history.can_go_back(),
                can_go_forward: self.history.can_go_forward(),
                path: self.history.current_path(),
                query: self.history.current_query(),
                prefix: self.history.current_prefix(),
                name_map: Arc::clone(&self.names),
                names,
                ..Default::default()
            },
        )
    }

    fn push(&mut self, target: NavigationTarget) -> Option<NavigationFailure> {
        match resolve_target(&self.names, &target) {
            Either::Left(Either::Left(p)) => self.history.push(p),
            Either::Left(Either::Right(n)) => return Some(NavigationFailure::Named(n)),
            Either::Right(e) => return self.external(e),
        }

        None
    }

    fn replace(&mut self, target: NavigationTarget) -> Option<NavigationFailure> {
        match resolve_target(&self.names, &target) {
            Either::Left(Either::Left(p)) => self.history.replace(p),
            Either::Left(Either::Right(n)) => return Some(NavigationFailure::Named(n)),
            Either::Right(e) => return self.external(e),
        }

        None
    }

    fn external(&mut self, external: String) -> Option<NavigationFailure> {
        match self.history.external(external.clone()) {
            true => None,
            false => Some(NavigationFailure::External(external)),
        }
    }

    fn subscribe(&mut self, id: Arc<I>) {
        self.subscribers.push(Arc::downgrade(&id));
        (self.subscriber_update)(id.as_ref().clone());
    }

    fn update_subscribers(&mut self) {
        let mut previous = Vec::new();
        self.subscribers.retain(|id| {
            if let Some(id) = id.upgrade() {
                if previous.contains(&id) {
                    false
                } else {
                    (self.subscriber_update)(id.as_ref().clone());
                    previous.push(id);
                    true
                }
            } else {
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    //! The tests for [`RouterService`] test various functions that are not exposed as public.
    //! However, several of those have an observable effect on the behavior of exposed functions.
    //!
    //! The alternative would be to send messages via the services channel and calling one of the
    //! `run` functions. However, for readability and clarity, it was chosen to directly call the
    //! private functions.

    use std::sync::Mutex;

    use crate::{
        history::MemoryHistory,
        routes::{ParameterRoute, Route, RouteContent},
    };

    use super::*;

    fn test_segment() -> Segment<&'static str> {
        Segment::content(RouteContent::Content(ContentAtom("index")))
            .fixed(
                "fixed",
                Route::content(RouteContent::Content(ContentAtom("fixed"))).name::<bool>(),
            )
            .fixed(
                "redirect",
                Route::content(RouteContent::Redirect(NavigationTarget::Internal(
                    String::from("fixed"),
                ))),
            )
            .fixed(
                "redirection-loop",
                Route::content(RouteContent::Redirect(NavigationTarget::Internal(
                    String::from("/redirection-loop"),
                ))),
            )
            .fixed(
                "%F0%9F%8E%BA",
                Route::content(RouteContent::Content(ContentAtom("🎺"))),
            )
            .catch_all(ParameterRoute::empty::<bool>())
    }

    #[test]
    fn new_provides_update_to_history() {
        struct TestHistory {}

        impl HistoryProvider for TestHistory {
            fn current_path(&self) -> String {
                todo!()
            }

            fn current_query(&self) -> Option<String> {
                todo!()
            }

            fn go_back(&mut self) {
                todo!()
            }

            fn go_forward(&mut self) {
                todo!()
            }

            fn push(&mut self, _path: String) {
                todo!()
            }

            fn replace(&mut self, _path: String) {
                todo!()
            }

            fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
                callback();
            }
        }

        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(TestHistory {}),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );

        assert!(matches!(
            s.receiver.try_next().unwrap().unwrap(),
            RouterMessage::Update
        ));
    }

    #[test]
    fn update_routing() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/fixed?test=value").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        assert_eq!(s.names, s.state.try_read().unwrap().name_map);
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert_eq!(state.query, Some(String::from("test=value")));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
        assert_eq!(s.names, state.name_map);
    }

    #[test]
    fn update_routing_root_index() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("index")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<RootIndex>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/"));
        assert!(state.query.is_none());
        assert!(state.prefix.is_none());
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn update_routing_redirect() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/redirect").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    #[should_panic = "reached redirect limit of 25"]
    #[cfg(debug_assertions)]
    fn update_routing_redirect_debug() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/redirection-loop").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn update_routing_redirect_release() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/redirection-loop").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("redirect limit")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<FailureRedirectionLimit>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/redirection-loop"));
        assert_eq!(state.can_go_back, false);
        assert_eq!(state.can_go_forward, false);
    }

    #[test]
    fn update_subscribers() {
        let ids = Arc::new(Mutex::new(Vec::new()));
        let ids2 = Arc::clone(&ids);

        let (mut s, _, _) = RouterService::<_, u8>::new(
            Segment::empty(),
            Box::<MemoryHistory>::default(),
            Arc::new(move |id| {
                ids2.lock().unwrap().push(id);
            }),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );

        let id0 = Arc::new(0);
        s.subscribe(Arc::clone(&id0));

        let id1 = Arc::new(1);
        s.subscribe(Arc::clone(&id1));

        let id1 = Arc::try_unwrap(id1).unwrap();
        s.update_subscribers();

        assert_eq!(s.subscribers.len(), 1);
        assert_eq!(s.subscribers[0].upgrade().unwrap(), id0);
        assert_eq!(*ids.lock().unwrap(), vec![*id0, id1, *id0]);
    }

    #[test]
    fn push_internal() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.push(NavigationTarget::Internal(String::from("/fixed")));
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert!(state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn push_named() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.push(NavigationTarget::named::<bool>());
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert!(state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn push_external() {
        let (mut s, tx, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();
        tx.unbounded_send(RouterMessage::Push(NavigationTarget::External(
            String::from("https://dioxuslabs.com/"),
        )))
        .unwrap();
        s.run_current();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("external target")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<FailureExternalNavigation>());
            r
        });
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(
                Name::of::<FailureExternalNavigation>(),
                String::from("https://dioxuslabs.com/"),
            );
            r
        });
        assert_eq!(state.path, String::from("/fixed"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn replace_named() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.replace(NavigationTarget::named::<bool>());
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn replace_internal() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.replace(NavigationTarget::Internal(String::from("/fixed")));
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("fixed")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<bool>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/fixed"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn replace_external() {
        let (mut s, tx, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();
        tx.unbounded_send(RouterMessage::Replace(NavigationTarget::External(
            String::from("https://dioxuslabs.com/"),
        )))
        .unwrap();
        s.run_current();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("external target")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<FailureExternalNavigation>());
            r
        });
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(
                Name::of::<FailureExternalNavigation>(),
                String::from("https://dioxuslabs.com/"),
            );
            r
        });
        assert_eq!(state.path, String::from("/fixed"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn subscribe() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            Segment::empty(),
            Box::<MemoryHistory>::default(),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );

        let id = Arc::new(0);
        s.subscribe(Arc::clone(&id));

        assert_eq!(s.subscribers.len(), 1);
        assert_eq!(s.subscribers[0].upgrade().unwrap(), id);
    }

    #[test]
    fn routing_callback() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let paths2 = Arc::clone(&paths);

        let (mut s, c, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/fixed").unwrap()),
            Arc::new(|_| {}),
            Some(Arc::new(move |state| {
                paths2.lock().unwrap().push(state.path.clone());
                Some("/%F0%9F%8E%BA".into())
            })),
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );

        assert!(paths.lock().unwrap().is_empty());

        s.init();
        assert_eq!(*paths.lock().unwrap(), vec![String::from("/fixed")]);

        c.unbounded_send(RouterMessage::Update).unwrap();
        s.run_current();
        assert_eq!(
            *paths.lock().unwrap(),
            vec![String::from("/fixed"), String::from("/%F0%9F%8E%BA")]
        );

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("🎺")])
    }

    #[test]
    fn url_decoding_do() {
        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/%F0%9F%A5%B3").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();

        let state = s.state.try_read().unwrap();
        assert!(state.content.is_empty());
        assert!(state.names.is_empty());
        assert_eq!(state.parameters, {
            let mut r = HashMap::new();
            r.insert(Name::of::<bool>(), String::from("🥳"));
            r
        });
        assert_eq!(state.path, String::from("/%F0%9F%A5%B3"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn url_decoding_do_not() {
        let (mut s, c, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(MemoryHistory::with_initial_path("/%F0%9F%8E%BA").unwrap()),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();
        c.unbounded_send(RouterMessage::Update).unwrap();
        s.run_current();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("🎺")]);
        assert!(state.names.is_empty());
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/%F0%9F%8E%BA"));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }

    #[test]
    fn prefix() {
        struct TestHistory {}

        impl HistoryProvider for TestHistory {
            fn current_path(&self) -> String {
                String::from("/")
            }

            fn current_query(&self) -> Option<String> {
                None
            }

            fn current_prefix(&self) -> Option<String> {
                Some(String::from("/prefix"))
            }

            fn can_go_back(&self) -> bool {
                false
            }

            fn can_go_forward(&self) -> bool {
                false
            }

            fn go_back(&mut self) {
                todo!()
            }

            fn go_forward(&mut self) {
                todo!()
            }

            fn push(&mut self, _path: String) {
                todo!()
            }

            fn replace(&mut self, _path: String) {
                todo!()
            }

            fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
                callback();
            }
        }

        let (mut s, _, _) = RouterService::<_, u8>::new(
            test_segment(),
            Box::new(TestHistory {}),
            Arc::new(|_| {}),
            None,
            ContentAtom("external target"),
            ContentAtom("named target"),
            ContentAtom("redirect limit"),
        );
        s.init();

        let state = s.state.try_read().unwrap();
        assert_eq!(state.content, vec![ContentAtom("index")]);
        assert_eq!(state.names, {
            let mut r = HashSet::new();
            r.insert(Name::of::<RootIndex>());
            r
        });
        assert!(state.parameters.is_empty());
        assert_eq!(state.path, String::from("/"));
        assert!(state.query.is_none());
        assert_eq!(state.prefix, Some(String::from("/prefix")));
        assert!(!state.can_go_back);
        assert!(!state.can_go_forward);
    }
}
