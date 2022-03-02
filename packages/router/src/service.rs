// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol

use crate::{cfg::RouterCfg, location::ParsedRoute};
use dioxus_core::ScopeId;
use futures_channel::mpsc::UnboundedSender;
use std::any::Any;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};
use url::Url;

/// An abstraction over the platform's history API.
///
/// The history is denoted using web-like semantics, with forward slashes delmitiing
/// routes and question marks denoting optional parameters.
///
/// This RouterService is exposed so you can modify the history directly. It
/// does not provide a high-level ergonomic API for your components. Instead,
/// you should consider using the components and hooks instead.
/// - [`Route`](struct.Route.html)
/// - [`Link`](struct.Link.html)
/// - [`UseRoute`](struct.UseRoute.html)
/// - [`Router`](struct.Router.html)
///
///
/// # Example
///
/// ```rust
/// let router = Router::new();
/// router.push_route("/home/custom");
/// cx.provide_context(router);
/// ```
///
/// # Platform Specific
///
/// - On the web, this is a [`BrowserHistory`](https://docs.rs/gloo/0.3.0/gloo/history/struct.BrowserHistory.html).
/// - On desktop, mobile, and SSR, this is just a Vec of Strings. Currently on
///   desktop, there is no way to tap into forward/back for the app unless explicitly set.
pub struct RouterCore {
    pub root_found: Cell<Option<ScopeId>>,

    pub stack: RefCell<Vec<Arc<ParsedRoute>>>,

    pub router_needs_update: Cell<bool>,

    pub tx: UnboundedSender<RouteEvent>,

    pub slots: Rc<RefCell<HashMap<ScopeId, String>>>,

    pub onchange_listeners: Rc<RefCell<HashSet<ScopeId>>>,

    pub query_listeners: Rc<RefCell<HashMap<ScopeId, String>>>,

    pub semgment_listeners: Rc<RefCell<HashMap<ScopeId, String>>>,

    pub history: Box<dyn RouterProvider>,

    pub cfg: RouterCfg,
}

pub type RouterService = Arc<RouterCore>;

#[derive(Debug)]
pub enum RouteEvent {
    Push(String),
    Pop,
}

impl RouterCore {
    pub fn new(tx: UnboundedSender<RouteEvent>, cfg: RouterCfg) -> Arc<Self> {
        #[cfg(feature = "web")]
        let history = Box::new(web::new(tx.clone()));

        #[cfg(not(feature = "web"))]
        let history = Box::new(hash::create_router());

        let route = Arc::new(ParsedRoute::new(history.init_location()));

        Arc::new(Self {
            cfg,
            tx,
            root_found: Cell::new(None),
            stack: RefCell::new(vec![route]),
            slots: Default::default(),
            semgment_listeners: Default::default(),
            query_listeners: Default::default(),
            onchange_listeners: Default::default(),
            history,
            router_needs_update: Default::default(),
        })
    }

    pub fn handle_route_event(&self, msg: RouteEvent) -> Option<Arc<ParsedRoute>> {
        log::debug!("handling route event {:?}", msg);
        self.root_found.set(None);

        match msg {
            RouteEvent::Push(route) => {
                let cur = self.current_location();

                let new_url = cur.url.join(&route).ok().unwrap();

                self.history.push(new_url.as_str());

                let route = Arc::new(ParsedRoute::new(new_url));

                self.stack.borrow_mut().push(route.clone());

                Some(route)
            }
            RouteEvent::Pop => {
                let mut stack = self.stack.borrow_mut();
                if stack.len() == 1 {
                    return None;
                }

                self.history.pop();
                stack.pop()
            }
        }
    }

    /// Push a new route to the history.
    ///
    /// This will trigger a route change event.
    ///
    /// This does not modify the current route
    pub fn push_route(&self, route: &str) {
        // convert the users route to our internal format
        self.tx
            .unbounded_send(RouteEvent::Push(route.to_string()))
            .unwrap();
    }

    /// Pop the current route from the history.
    ///
    ///
    pub fn pop_route(&self) {
        self.tx.unbounded_send(RouteEvent::Pop).unwrap();
    }

    pub(crate) fn register_total_route(&self, route: String, scope: ScopeId) {
        let clean = clean_route(route);
        self.slots.borrow_mut().insert(scope, clean);
    }

    pub(crate) fn should_render(&self, scope: ScopeId) -> bool {
        log::debug!("Checking render: {:?}", scope);

        if let Some(root_id) = self.root_found.get() {
            return root_id == scope;
        }

        let roots = self.slots.borrow();

        if let Some(route) = roots.get(&scope) {
            log::debug!("Registration found for scope {:?} {:?}", scope, route);

            if route_matches_path(&self.current_location(), route) || route.is_empty() {
                self.root_found.set(Some(scope));
                true
            } else {
                false
            }
        } else {
            log::debug!("no route found for scope: {:?}", scope);
            false
        }
    }

    /// Get the current location of the Router
    pub fn current_location(&self) -> Arc<ParsedRoute> {
        self.stack.borrow().last().unwrap().clone()
    }

    pub fn query_current_location(&self) -> HashMap<String, String> {
        todo!()
        // self.history.borrow().query()
    }

    /// Get the current location of the Router
    pub fn native_location<T: 'static>(&self) -> Option<Box<T>> {
        self.history.native_location().downcast::<T>().ok()
    }

    /// Registers a scope to regenerate on route change.
    ///
    /// This is useful if you've built some abstraction on top of the router service.
    pub fn subscribe_onchange(&self, id: ScopeId) {
        self.onchange_listeners.borrow_mut().insert(id);
    }

    /// Unregisters a scope to regenerate on route change.
    ///
    /// This is useful if you've built some abstraction on top of the router service.
    pub fn unsubscribe_onchange(&self, id: ScopeId) {
        self.onchange_listeners.borrow_mut().remove(&id);
    }
}

fn clean_route(route: String) -> String {
    if route.as_str() == "/" {
        return route;
    }
    route.trim_end_matches('/').to_string()
}

fn clean_path(path: &str) -> &str {
    if path == "/" {
        return path;
    }
    let sub = path.trim_end_matches('/');

    if sub.starts_with('/') {
        &path[1..]
    } else {
        sub
    }
}

fn route_matches_path(cur: &ParsedRoute, attempt: &str) -> bool {
    let cur_pieces = cur.url.path_segments().unwrap().collect::<Vec<_>>();
    let attempt_pieces = clean_path(attempt).split('/').collect::<Vec<_>>();

    if attempt == "/" && cur_pieces.len() == 1 && cur_pieces[0].is_empty() {
        return true;
    }

    log::debug!(
        "Comparing cur {:?} to attempt {:?}",
        cur_pieces,
        attempt_pieces
    );

    if attempt_pieces.len() != cur_pieces.len() {
        return false;
    }

    for (i, r) in attempt_pieces.iter().enumerate() {
        log::debug!("checking route: {:?}", r);

        // If this is a parameter then it matches as long as there's
        // _any_thing in that spot in the path.
        if r.starts_with(':') {
            continue;
        }

        if cur_pieces[i] != *r {
            return false;
        }
    }

    true
}

pub trait RouterProvider {
    fn push(&self, path: &str);
    fn pop(&self);
    fn native_location(&self) -> Box<dyn Any>;
    fn init_location(&self) -> Url;
}

mod hash {
    use super::*;

    /// a simple cross-platform hash-based router
    pub struct HashRouter {}

    impl RouterProvider for HashRouter {
        fn push(&self, _path: &str) {}

        fn native_location(&self) -> Box<dyn Any> {
            Box::new(())
        }

        fn pop(&self) {}

        fn init_location(&self) -> Url {
            Url::parse("app:///").unwrap()
        }
    }
}

#[cfg(feature = "web")]
mod web {
    use super::RouterProvider;
    use crate::RouteEvent;

    use futures_channel::mpsc::UnboundedSender;
    use gloo::{
        events::EventListener,
        history::{BrowserHistory, History},
    };
    use std::any::Any;
    use url::Url;

    pub struct WebRouter {
        // keep it around so it drops when the router is dropped
        _listener: gloo::events::EventListener,

        history: BrowserHistory,
    }

    impl RouterProvider for WebRouter {
        fn push(&self, path: &str) {
            self.history.push(path);
            // use gloo::history;
            // web_sys::window()
            //     .unwrap()
            //     .location()
            //     .set_href(path)
            //     .unwrap();
        }

        fn native_location(&self) -> Box<dyn Any> {
            todo!()
        }

        fn pop(&self) {
            // set the title, maybe?
        }

        fn init_location(&self) -> Url {
            url::Url::parse(&web_sys::window().unwrap().location().href().unwrap()).unwrap()
        }
    }

    pub fn new(tx: UnboundedSender<RouteEvent>) -> WebRouter {
        WebRouter {
            history: BrowserHistory::new(),
            _listener: EventListener::new(&web_sys::window().unwrap(), "popstate", move |_| {
                let _ = tx.unbounded_send(RouteEvent::Pop);
            }),
        }
    }
}
