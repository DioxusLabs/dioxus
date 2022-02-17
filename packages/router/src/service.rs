// todo: how does router work in multi-window contexts?
// does each window have its own router? probably, lol

use dioxus_core::ScopeId;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

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
pub struct RouterService {
    pub(crate) regen_route: Rc<dyn Fn(ScopeId)>,

    pub(crate) pending_events: Rc<RefCell<Vec<RouteEvent>>>,

    slots: Rc<RefCell<Vec<(ScopeId, String)>>>,

    onchange_listeners: Rc<RefCell<HashSet<ScopeId>>>,

    root_found: Rc<Cell<Option<ScopeId>>>,

    cur_path_params: Rc<RefCell<HashMap<String, String>>>,

    history: Box<dyn RouterProvider>,
}

pub(crate) enum RouteEvent {
    Change,
    Pop,
    Push,
}

impl RouterService {
    /// Creates a new RouterService.
    ///
    /// Takes a callback that can regenerated *any* scope and the root scope
    /// of the router itself.
    ///
    /// In most cases, `root_scope` should be `ScopeId(0)`.
    pub fn new(regen_route: Rc<dyn Fn(ScopeId)>, root_scope: ScopeId) -> Self {
        let onchange_listeners = Rc::new(RefCell::new(HashSet::new()));
        let slots: Rc<RefCell<Vec<(ScopeId, String)>>> = Default::default();
        let pending_events: Rc<RefCell<Vec<RouteEvent>>> = Default::default();
        let root_found = Rc::new(Cell::new(None));

        // let mut history: Box<dyn RouterProvider> = if cfg!(feature = "web") {
        //     use gloo::history::{BrowserHistory, History, HistoryListener, Location};
        //     let history = BrowserHistory::default();
        //     let location = history.location();
        //     let path = location.path();
        //     let listener = history.listen({
        //         dioxus_core::to_owned![
        //             pending_events,
        //             regen_route,
        //             root_found,
        //             slots,
        //             onchange_listeners
        //         ];
        //         move || {
        //             root_found.set(None);
        //             // checking if the route is valid is cheap, so we do it
        //             for (slot, root) in slots.borrow_mut().iter().rev() {
        //                 regen_route(*slot);
        //             }

        //             for listener in onchange_listeners.borrow_mut().iter() {
        //                 regen_route(*listener);
        //             }

        //             // also regenerate the root
        //             regen_route(root_scope);

        //             pending_events.borrow_mut().push(RouteEvent::Change)
        //         }
        //     });

        //     Box::new(web::create_router())
        // } else {
        //     Box::new(hash::create_router())
        // };

        let mut history = Box::new(hash::create_router());

        Self {
            root_found,
            history,
            regen_route,
            slots,
            pending_events,
            onchange_listeners,
            cur_path_params: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Push a new route to the history.
    ///
    /// This will trigger a route change event.
    ///
    /// This does not modify the current route
    pub fn push_route(&self, route: &str) {
        self.history.push(route);
    }

    pub(crate) fn register_total_route(&self, route: String, scope: ScopeId, fallback: bool) {
        let clean = clean_route(route);
        self.slots.borrow_mut().push((scope, clean));
    }

    pub(crate) fn should_render(&self, scope: ScopeId) -> bool {
        if let Some(root_id) = self.root_found.get() {
            if root_id == scope {
                return true;
            }
            return false;
        }

        let path = self.history.path();

        let roots = self.slots.borrow();

        let root = roots.iter().find(|(id, route)| id == &scope);

        // fallback logic
        match root {
            Some((id, route)) => {
                if let Some(params) = route_matches_path(route, &path) {
                    self.root_found.set(Some(*id));
                    *self.cur_path_params.borrow_mut() = params;
                    true
                } else if route.is_empty() {
                    self.root_found.set(Some(*id));
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Get the current location of the Router
    pub fn current_location(&self) -> Rc<str> {
        self.history.path()
    }

    pub fn query_current_location(&self) -> HashMap<String, String> {
        todo!()
        // self.history.borrow().query()
    }

    /// Get the current location of the Router
    pub fn native_location<T: 'static>(&self) -> Option<Box<T>> {
        self.history.native_location().downcast::<T>().ok()
    }

    /// Get the current params of the router
    pub fn current_path_params(&self) -> Ref<HashMap<String, String>> {
        self.cur_path_params.borrow()
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
    path.trim_end_matches('/')
}

fn route_matches_path(route: &str, path: &str) -> Option<HashMap<String, String>> {
    let route_pieces = route.split('/').collect::<Vec<_>>();
    let path_pieces = clean_path(path).split('/').collect::<Vec<_>>();

    if route_pieces.len() != path_pieces.len() {
        return None;
    }

    let mut matches = HashMap::new();
    for (i, r) in route_pieces.iter().enumerate() {
        // If this is a parameter then it matches as long as there's
        // _any_thing in that spot in the path.
        if r.starts_with(':') {
            let param = &r[1..];
            matches.insert(param.to_string(), path_pieces[i].to_string());
            continue;
        }

        if path_pieces[i] != *r {
            return None;
        }
    }

    Some(matches)
}

use std::any::Any;

use dioxus_core::ScopeState;

pub(crate) trait RouterProvider {
    fn push(&self, path: &str);
    fn path(&self) -> Rc<str>;
    fn listen(&self, callback: Box<dyn Fn()>);
    fn native_location(&self) -> Box<dyn Any>;
}

mod hash {
    use super::*;
    use dioxus_core::ScopeState;

    /// a simple cross-platform hash-based router
    pub struct HashRouter {}

    impl RouterProvider for HashRouter {
        fn push(&self, path: &str) {}

        fn path(&self) -> Rc<str> {
            "/home".into()
        }

        fn listen(&self, callback: Box<dyn Fn()>) {}

        fn native_location(&self) -> Box<dyn Any> {
            Box::new(())
        }
    }

    pub(crate) fn create_router() -> HashRouter {
        HashRouter {}
    }
}

#[cfg(feature = "web")]
mod web {
    use super::RouterProvider;
    use crate::RouteEvent;
    use dioxus_core::{ScopeId, ScopeState};
    use gloo::history::HistoryResult;
    use gloo::history::{BrowserHistory, History, HistoryListener, Location};
    use std::any::Any;
    use std::rc::Rc;

    pub struct WebRouter {}

    impl RouterProvider for WebRouter {
        fn path(&self) -> Rc<str> {
            unimplemented!()
        }

        fn listen(&self, callback: Box<dyn Fn()>) {
            unimplemented!()
        }

        fn push(&self, path: &str) {
            todo!()
        }

        fn native_location(&self) -> Box<dyn Any> {
            todo!()
        }
    }

    pub fn create_router() -> WebRouter {
        // let history = BrowserHistory::default();
        // let location = history.location();
        // let path = location.path();

        // let onchange_listeners = Rc::new(RefCell::new(HashSet::new()));
        // let slots: Rc<RefCell<Vec<(ScopeId, String)>>> = Default::default();
        // let pending_events: Rc<RefCell<Vec<RouteEvent>>> = Default::default();
        // let root_found = Rc::new(Cell::new(None));

        // let listener = history.listen({
        //     let pending_events = pending_events.clone();
        //     let regen_route = regen_route.clone();
        //     let root_found = root_found.clone();
        //     let slots = slots.clone();
        //     let onchange_listeners = onchange_listeners.clone();
        //     move || {
        //         root_found.set(None);
        //         // checking if the route is valid is cheap, so we do it
        //         for (slot, root) in slots.borrow_mut().iter().rev() {
        //             regen_route(*slot);
        //         }

        //         for listener in onchange_listeners.borrow_mut().iter() {
        //             regen_route(*listener);
        //         }

        //         // also regenerate the root
        //         regen_route(root_scope);

        //         pending_events.borrow_mut().push(RouteEvent::Change)
        //     }
        // });

        todo!()
    }
}
