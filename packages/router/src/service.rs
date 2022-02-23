use gloo::history::{BrowserHistory, History, HistoryListener, Location};
use std::{
    cell::{Cell, Ref, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc, sync::Arc,
};

use dioxus_core::ScopeId;

use crate::platform::RouterProvider;

pub struct RouterService {
    pub(crate) regen_route: Arc<dyn Fn(ScopeId)>,
    pub(crate) pending_events: Rc<RefCell<Vec<RouteEvent>>>,
    slots: Rc<RefCell<Vec<(ScopeId, String)>>>,
    onchange_listeners: Rc<RefCell<HashSet<ScopeId>>>,
    root_found: Rc<Cell<Option<ScopeId>>>,
    cur_path_params: Rc<RefCell<HashMap<String, String>>>,

    // history: Rc<dyn RouterProvider>,
    history: Rc<RefCell<BrowserHistory>>,
    listener: HistoryListener,
}

pub enum RouteEvent {
    Change,
    Pop,
    Push,
}

enum RouteSlot {
    Routes {
        // the partial route
        partial: String,

        // the total route
        total: String,

        // Connections to other routs
        rest: Vec<RouteSlot>,
    },
}

impl RouterService {
    pub fn new(regen_route: Arc<dyn Fn(ScopeId)>, root_scope: ScopeId) -> Self {
        let history = BrowserHistory::default();
        let location = history.location();
        let path = location.path();

        let onchange_listeners = Rc::new(RefCell::new(HashSet::new()));
        let slots: Rc<RefCell<Vec<(ScopeId, String)>>> = Default::default();
        let pending_events: Rc<RefCell<Vec<RouteEvent>>> = Default::default();
        let root_found = Rc::new(Cell::new(None));

        let listener = history.listen({
            let pending_events = pending_events.clone();
            let regen_route = regen_route.clone();
            let root_found = root_found.clone();
            let slots = slots.clone();
            let onchange_listeners = onchange_listeners.clone();
            move || {
                root_found.set(None);
                // checking if the route is valid is cheap, so we do it
                for (slot, root) in slots.borrow_mut().iter().rev() {
                    regen_route(*slot);
                }

                for listener in onchange_listeners.borrow_mut().iter() {
                    regen_route(*listener);
                }

                // also regenerate the root
                regen_route(root_scope);

                pending_events.borrow_mut().push(RouteEvent::Change)
            }
        });

        Self {
            listener,
            root_found,
            history: Rc::new(RefCell::new(history)),
            regen_route,
            slots,
            pending_events,
            onchange_listeners,
            cur_path_params: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn push_route(&self, route: &str) {
        self.history.borrow_mut().push(route);
    }

    pub fn register_total_route(&self, route: String, scope: ScopeId, fallback: bool) {
        let clean = clean_route(route);
        self.slots.borrow_mut().push((scope, clean));
    }

    pub fn should_render(&self, scope: ScopeId) -> bool {
        if let Some(root_id) = self.root_found.get() {
            if root_id == scope {
                return true;
            }
            return false;
        }

        let location = self.history.borrow().location();
        let path = location.path();

        let roots = self.slots.borrow();

        let root = roots.iter().find(|(id, route)| id == &scope);

        // fallback logic
        match root {
            Some((id, route)) => {
                if let Some(params) = route_matches_path(route, path) {
                    self.root_found.set(Some(*id));
                    *self.cur_path_params.borrow_mut() = params;
                    true
                } else {
                    if route == "" {
                        self.root_found.set(Some(*id));
                        true
                    } else {
                        false
                    }
                }
            }
            None => false,
        }
    }

    pub fn current_location(&self) -> Location {
        self.history.borrow().location().clone()
    }

    pub fn current_path_params(&self) -> Ref<HashMap<String, String>> {
        self.cur_path_params.borrow()
    }

    pub fn subscribe_onchange(&self, id: ScopeId) {
        self.onchange_listeners.borrow_mut().insert(id);
    }

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

pub struct RouterCfg {
    initial_route: String,
}

impl RouterCfg {
    pub fn new(initial_route: String) -> Self {
        Self { initial_route }
    }
}
