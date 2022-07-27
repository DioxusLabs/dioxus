// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol
// maybe every window renders based on the router and we have a single DOM for all of them? :P

use crate::cfg::RouterCfg;
use dioxus::core::{ScopeId, ScopeState, VirtualDom};
use std::any::Any;
use std::sync::Weak;
use std::{
    cell::RefCell,
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
    pub(crate) route_found: Rc<RefCell<HashMap<ScopeId, ScopeId>>>,

    pub(crate) stack: RefCell<Vec<Arc<ParsedRoute>>>,

    pub(crate) slots: Rc<RefCell<HashMap<ScopeId, String>>>,

    pub(crate) ordering: Rc<RefCell<HashMap<ScopeId, Vec<ScopeId>>>>,

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

        let route = Arc::new(history.init_location());

        let svc = Arc::new(Self {
            cfg,
            regen_any_route: cx.schedule_update_any(),
            router_id: cx.scope_id(),
            route_found: Default::default(),
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
        self.route_found.borrow_mut().clear();

        (self.regen_any_route)(self.router_id);

        for listener in self.onchange_listeners.borrow().iter() {
            (self.regen_any_route)(*listener);
        }

        // we don't really care for order when sending to renderer
        // so we could use self.slots.keys() instead?
        for parent in self.ordering.borrow().iter() {
            for route in parent.1.iter() {
                (self.regen_any_route)(*route);
            }
        }
    }

    /// Get the current location of the Router
    pub fn current_location(&self) -> Arc<ParsedRoute> {
        let resp = self.stack.borrow().last().unwrap().clone();
        return resp;
    }

    /// Get the current route of the Router
    pub fn current_route(&self) -> String {
        self.current_location().url.path().to_string()
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

    pub(crate) fn register_total_route(
        &self,
        route: String,
        parent: Option<ScopeId>,
        scope: ScopeId,
    ) {
        let clean = clean_route(route);

        // block in place to release borrows
        {
            let parent_id = parent.unwrap_or_else(|| self.router_id);
            let mut ordering_parent = self.ordering.borrow_mut();
            let mut slots = self.slots.borrow_mut();
            let ordering = ordering_parent.entry(parent_id).or_insert(Vec::new());
            // Here we are only checking for route sibligns, so nesting should be efficient.
            let index = ordering.iter().position(|&r| {
                if let Some(item) = slots.get(&r) {
                    item.len() <= clean.len()
                } else {
                    false
                }
            });
            if let Some(found_index) = index {
                ordering.insert(found_index, scope);
            } else {
                ordering.push(scope);
            }
            slots.insert(scope, clean.clone());
        }

        // if the route we are adding would be a candidate to be rendered we need to reevaluate
        if route_matches_path(
            &self.current_location().url,
            clean.as_str(),
            self.cfg.base_url.as_ref(),
        ) || clean.is_empty()
        {
            self.regen_routes();
        }
    }

    pub(crate) fn unregister_total_route(&self, scope_id: ScopeId, parent: Option<ScopeId>) {
        let parent_id = parent.unwrap_or_else(|| self.router_id);

        // block in place to release borrows
        {
            let mut ordering = self.ordering.borrow_mut();
            // we remove route from parent
            ordering.entry(parent_id).and_modify(|items| {
                items.retain(|&r| r != scope_id);
            });
            // and we remove route AS parent
            ordering.remove(&scope_id);
            self.slots.borrow_mut().remove(&scope_id);
        }

        // If the route we are removing is currently rendered we need to reevaluate
        let current_route = self.route_found.borrow().get(&parent_id).cloned();
        if let Some(root_id) = current_route {
            if root_id == scope_id {
                self.regen_routes();
            }
        }
    }

    pub(crate) fn should_render(&self, scope: ScopeId, parent: Option<ScopeId>) -> bool {
        // rendering is done one level at a time. When a route is active for a particular level,
        // every child route will be checked after render. That's the reason to have the
        // `route_found` momoized considering the route parent.
        let parent_id = parent.unwrap_or_else(|| self.router_id);
        if let Some(root_id) = self.route_found.borrow().get(&parent_id) {
            return root_id == &scope;
        }

        let roots = self.slots.borrow();
        // When checking if a route needs to be rendered, we actually check every route inside parent
        // to see which one would be the best candidate. When found, we store the value so that
        // every sibling route will avoid this check.
        for ordered_scope in self.ordering.borrow().get(&parent_id).unwrap() {
            if let Some(route) = roots.get(ordered_scope) {
                let cur = &self.current_location().url;
                log::trace!("Checking if {} matches {}", cur, route);

                if route_matches_path(cur, route, self.cfg.base_url.as_ref()) || route.is_empty() {
                    self.route_found
                        .borrow_mut()
                        .insert(parent_id, *ordered_scope);
                    return ordered_scope == &scope;
                }
            }
        }
        return false;
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

    // allows for empty paths (404)
    if cur_pieces.len() == 0 {
        return false;
    }

    if attempt == "/" && cur_pieces.len() == 1 && cur_pieces[0].is_empty() {
        return true;
    }

    // allow slashes at the end of the path
    if cur_pieces.last() == Some(&"") {
        cur_pieces.pop();
    }

    let attempt_pieces = clean_path(attempt).split('/').collect::<Vec<_>>();

    for (i, r) in attempt_pieces.iter().enumerate() {
        // Stop matching if the route does not have enough segments
        if i >= cur_pieces.len() {
            return false;
        }
        // If this is a parameter then it matches as long as there's
        // _any_thing in that spot in the path.
        if r.starts_with(':') {
            // If the parameter to match is empty we skip the match
            // Ideas:
            //   - we could have r.ends_with('?') to allow for optional param?
            //   - we could do some regexp thing to match like /:id(\d+) maybe?
            if cur_pieces[i].is_empty() {
                return false;
            }
            continue;
        }

        if cur_pieces[i] != *r {
            return false;
        }
    }

    // NOTE: We can still have pieces left unchecked, but we can't avoid it in every case,
    // as it would break nested routes (/blog -> /:id needs to render for /blog and then check children)
    // Maybe we could add an exact param on routes to enforce that no more pieces are left?

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
