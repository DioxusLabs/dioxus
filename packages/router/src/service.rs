// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol

use crate::cfg::RouterCfg;
use dioxus::core::{ScopeId, ScopeState, VirtualDom};
use std::any::Any;
use std::sync::Weak;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    str::FromStr,
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
/// ```rust, ignore
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
    pub(crate) route_found: Cell<Option<ScopeId>>,

    pub(crate) stack: RefCell<Vec<Arc<ParsedRoute>>>,

    pub(crate) slots: Rc<RefCell<HashMap<ScopeId, String>>>,

    pub(crate) ordering: Rc<RefCell<Vec<ScopeId>>>,

    pub(crate) onchange_listeners: Rc<RefCell<HashSet<ScopeId>>>,

    pub(crate) history: Box<dyn RouterProvider>,

    pub(crate) regen_any_route: Arc<dyn Fn(ScopeId)>,

    pub(crate) router_id: ScopeId,

    pub(crate) cfg: RouterCfg,
}

/// A shared type for the RouterCore.
pub type RouterService = Arc<RouterCore>;

/// A route is a combination of window title, saved state, and a URL.
#[derive(Debug, Clone)]
pub struct ParsedRoute {
    /// The URL of the route.
    pub url: Url,

    /// The title of the route.
    pub title: Option<String>,

    /// The serialized state of the route.
    pub serialized_state: Option<String>,
}

impl RouterCore {
    pub(crate) fn new(cx: &ScopeState, cfg: RouterCfg) -> Arc<Self> {
        #[cfg(feature = "web")]
        let history = Box::new(web::new());

        #[cfg(not(feature = "web"))]
        let history = Box::new(hash::new());

        let route = match &cfg.initial_url {
            Some(url) => Arc::new(ParsedRoute {
                url: Url::from_str(url).unwrap_or_else(|_|
                    panic!(
                        "RouterCfg expects a valid initial_url, but got '{}'. Example: '{{scheme}}://{{?authority}}/{{?path}}'",
                        &url
                    )
                ),
                title: None,
                serialized_state: None,
            }),
            None => Arc::new(history.init_location()),
        };

        let svc = Arc::new(Self {
            cfg,
            regen_any_route: cx.schedule_update_any(),
            router_id: cx.scope_id(),
            route_found: Cell::new(None),
            stack: RefCell::new(vec![route]),
            ordering: Default::default(),
            slots: Default::default(),
            onchange_listeners: Default::default(),
            history,
        });

        svc.history.attach_listeners(Arc::downgrade(&svc));

        svc
    }

    /// Push a new route with no custom title or serialized state.
    ///
    /// This is a convenience method for easily navigating.
    pub fn navigate_to(&self, route: &str) {
        self.push_route(route, None, None);
    }

    /// Push a new route to the history.
    ///
    /// This will trigger a route change event.
    ///
    /// This does not modify the current route
    pub fn push_route(&self, route: &str, title: Option<String>, serialized_state: Option<String>) {
        let new_route = Arc::new(ParsedRoute {
            url: self.current_location().url.join(route).ok().unwrap(),
            title,
            serialized_state,
        });

        self.history.push(&new_route);
        self.stack.borrow_mut().push(new_route);

        self.regen_routes();
    }

    /// Instead of pushing a new route, replaces the current route.
    pub fn replace_route(
        &self,
        route: &str,
        title: Option<String>,
        serialized_state: Option<String>,
    ) {
        let new_route = Arc::new(ParsedRoute {
            url: self.current_location().url.join(route).ok().unwrap(),
            title,
            serialized_state,
        });

        self.history.replace(&new_route);
        *self.stack.borrow_mut().last_mut().unwrap() = new_route;

        self.regen_routes();
    }

    /// Pop the current route from the history.
    pub fn pop_route(&self) {
        let mut stack = self.stack.borrow_mut();

        if stack.len() > 1 {
            stack.pop();
        }

        self.regen_routes();
    }

    /// Regenerate any routes that need to be regenerated, discarding the currently found route
    ///
    /// You probably don't need this method
    pub fn regen_routes(&self) {
        self.route_found.set(None);

        (self.regen_any_route)(self.router_id);

        for listener in self.onchange_listeners.borrow().iter() {
            (self.regen_any_route)(*listener);
        }

        for route in self.ordering.borrow().iter().rev() {
            (self.regen_any_route)(*route);
        }
    }

    /// Get the current location of the Router
    pub fn current_location(&self) -> Arc<ParsedRoute> {
        self.stack.borrow().last().unwrap().clone()
    }

    /// Get the current native location of the Router
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

    pub(crate) fn register_total_route(&self, route: String, scope: ScopeId) {
        let clean = clean_route(route);
        self.slots.borrow_mut().insert(scope, clean);
        self.ordering.borrow_mut().push(scope);
    }

    pub(crate) fn should_render(&self, scope: ScopeId) -> bool {
        if let Some(root_id) = self.route_found.get() {
            return root_id == scope;
        }

        let roots = self.slots.borrow();

        if let Some(route) = roots.get(&scope) {
            let cur = &self.current_location().url;
            log::trace!("Checking if {} matches {}", cur, route);

            if route_matches_path(cur, route, self.cfg.base_url.as_ref()) || route.is_empty() {
                self.route_found.set(Some(scope));
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Get the router service from an existing VirtualDom.
///
/// Takes an optional target_scope parameter to specify the scope to use if ScopeId is not the component
/// that owns the router.
///
/// This might change in the future.
pub fn get_router_from_vdom(
    dom: &VirtualDom,
    target_scope: Option<ScopeId>,
) -> Option<Arc<RouterCore>> {
    dom.get_scope(target_scope.unwrap_or(ScopeId(0)))
        .and_then(|scope| scope.consume_context::<Arc<RouterCore>>())
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

fn route_matches_path(cur: &Url, attempt: &str, base_url: Option<&String>) -> bool {
    let cur_piece_iter = cur.path_segments().unwrap();

    let mut cur_pieces = match base_url {
        // baseurl is naive right now and doesn't support multiple nesting levels
        Some(_) => cur_piece_iter.skip(1).collect::<Vec<_>>(),
        None => cur_piece_iter.collect::<Vec<_>>(),
    };

    if attempt == "/" && cur_pieces.len() == 1 && cur_pieces[0].is_empty() {
        return true;
    }

    // allow slashes at the end of the path
    if cur_pieces.last() == Some(&"") {
        cur_pieces.pop();
    }

    let attempt_pieces = clean_path(attempt).split('/').collect::<Vec<_>>();

    if attempt_pieces.len() != cur_pieces.len() {
        return false;
    }

    for (i, r) in attempt_pieces.iter().enumerate() {
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

pub(crate) trait RouterProvider {
    fn push(&self, route: &ParsedRoute);
    fn replace(&self, route: &ParsedRoute);
    fn native_location(&self) -> Box<dyn Any>;
    fn init_location(&self) -> ParsedRoute;
    fn attach_listeners(&self, svc: Weak<RouterCore>);
}

#[cfg(not(feature = "web"))]
mod hash {
    use super::*;

    pub fn new() -> HashRouter {
        HashRouter {}
    }

    /// a simple cross-platform hash-based router
    pub struct HashRouter {}

    impl RouterProvider for HashRouter {
        fn push(&self, _route: &ParsedRoute) {}

        fn native_location(&self) -> Box<dyn Any> {
            Box::new(())
        }

        fn init_location(&self) -> ParsedRoute {
            ParsedRoute {
                url: Url::parse("app:///").unwrap(),
                title: None,
                serialized_state: None,
            }
        }

        fn replace(&self, _route: &ParsedRoute) {}

        fn attach_listeners(&self, _svc: Weak<RouterCore>) {}
    }
}

#[cfg(feature = "web")]
mod web {
    use super::RouterProvider;
    use crate::ParsedRoute;

    use gloo_events::EventListener;
    use std::{any::Any, cell::Cell};
    use web_sys::History;

    pub struct WebRouter {
        // keep it around so it drops when the router is dropped
        _listener: Cell<Option<gloo_events::EventListener>>,

        window: web_sys::Window,
        history: History,
    }

    impl RouterProvider for WebRouter {
        fn push(&self, route: &ParsedRoute) {
            let ParsedRoute {
                url,
                title,
                serialized_state,
            } = route;

            let _ = self.history.push_state_with_url(
                &wasm_bindgen::JsValue::from_str(serialized_state.as_deref().unwrap_or("")),
                title.as_deref().unwrap_or(""),
                Some(url.as_str()),
            );
        }

        fn replace(&self, route: &ParsedRoute) {
            let ParsedRoute {
                url,
                title,
                serialized_state,
            } = route;

            let _ = self.history.replace_state_with_url(
                &wasm_bindgen::JsValue::from_str(serialized_state.as_deref().unwrap_or("")),
                title.as_deref().unwrap_or(""),
                Some(url.as_str()),
            );
        }

        fn native_location(&self) -> Box<dyn Any> {
            Box::new(self.window.location())
        }

        fn init_location(&self) -> ParsedRoute {
            ParsedRoute {
                url: url::Url::parse(&web_sys::window().unwrap().location().href().unwrap())
                    .unwrap(),
                title: web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .title()
                    .into(),
                serialized_state: None,
            }
        }

        fn attach_listeners(&self, svc: std::sync::Weak<crate::RouterCore>) {
            self._listener.set(Some(EventListener::new(
                &web_sys::window().unwrap(),
                "popstate",
                move |_| {
                    if let Some(svc) = svc.upgrade() {
                        svc.pop_route();
                    }
                },
            )));
        }
    }

    pub(crate) fn new() -> WebRouter {
        WebRouter {
            history: web_sys::window().unwrap().history().unwrap(),
            window: web_sys::window().unwrap(),
            _listener: Cell::new(None),
        }
    }
}
