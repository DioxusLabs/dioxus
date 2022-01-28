use gloo::history::{BrowserHistory, History, HistoryListener, Location};
use std::{
    cell::{Cell, Ref, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

use dioxus_core::ScopeId;

pub struct RouterService {
    pub(crate) regen_route: Rc<dyn Fn(ScopeId)>,
    pub(crate) pending_events: Rc<RefCell<Vec<RouteEvent>>>,
    history: Rc<RefCell<BrowserHistory>>,
    slots: Rc<RefCell<Vec<(ScopeId, String)>>>,
    onchange_listeners: Rc<RefCell<HashSet<ScopeId>>>,
    root_found: Rc<Cell<Option<ScopeId>>>,
    cur_path_params: Rc<RefCell<HashMap<String, String>>>,
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
    pub fn new(regen_route: Rc<dyn Fn(ScopeId)>, root_scope: ScopeId) -> Self {
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
                    log::trace!("regenerating slot {:?} for root '{}'", slot, root);
                    regen_route(*slot);
                }

                for listener in onchange_listeners.borrow_mut().iter() {
                    log::trace!("regenerating listener {:?}", listener);
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
        log::trace!("Pushing route: {}", route);
        self.history.borrow_mut().push(route);
    }

    pub fn register_total_route(&self, route: String, scope: ScopeId, fallback: bool) {
        let clean = clean_route(route);
        log::trace!("Registered route '{}' with scope id {:?}", clean, scope);
        self.slots.borrow_mut().push((scope, clean));
    }

    pub fn should_render(&self, scope: ScopeId) -> bool {
        log::trace!("Should render scope id {:?}?", scope);
        if let Some(root_id) = self.root_found.get() {
            log::trace!("  we already found a root with scope id {:?}", root_id);
            if root_id == scope {
                log::trace!("    yes - it's a match");
                return true;
            }
            log::trace!("    no - it's not a match");
            return false;
        }

        let location = self.history.borrow().location();
        let path = location.path();
        log::trace!("  current path is '{}'", path);

        let roots = self.slots.borrow();

        let root = roots.iter().find(|(id, route)| id == &scope);

        // fallback logic
        match root {
            Some((id, route)) => {
                log::trace!(
                    "  matched given scope id {:?} with route root '{}'",
                    scope,
                    route,
                );
                if let Some(params) = route_matches_path(route, path) {
                    log::trace!("    and it matches the current path '{}'", path);
                    self.root_found.set(Some(*id));
                    *self.cur_path_params.borrow_mut() = params;
                    true
                } else {
                    if route == "" {
                        log::trace!("    and the route is the root, so we will use that without a better match");
                        self.root_found.set(Some(*id));
                        true
                    } else {
                        log::trace!("    and the route '{}' is not the root nor does it match the current path", route);
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
        log::trace!("Subscribing onchange for scope id {:?}", id);
        self.onchange_listeners.borrow_mut().insert(id);
    }

    pub fn unsubscribe_onchange(&self, id: ScopeId) {
        log::trace!("Subscribing onchange for scope id {:?}", id);
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

    log::trace!(
        "  checking route pieces {:?} vs path pieces {:?}",
        route_pieces,
        path_pieces,
    );

    if route_pieces.len() != path_pieces.len() {
        log::trace!("    the routes are different lengths");
        return None;
    }

    let mut matches = HashMap::new();
    for (i, r) in route_pieces.iter().enumerate() {
        log::trace!("    checking route piece '{}' vs path", r);
        // If this is a parameter then it matches as long as there's
        // _any_thing in that spot in the path.
        if r.starts_with(':') {
            log::trace!(
                "      route piece '{}' starts with a colon so it matches anything",
                r,
            );
            let param = &r[1..];
            matches.insert(param.to_string(), path_pieces[i].to_string());
            continue;
        }
        log::trace!(
            "      route piece '{}' must be an exact match for path piece '{}'",
            r,
            path_pieces[i],
        );
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
