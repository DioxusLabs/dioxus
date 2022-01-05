use gloo::history::{BrowserHistory, History, HistoryListener};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use dioxus_core::ScopeId;

pub struct RouterService {
    pub(crate) regen_route: Rc<dyn Fn(ScopeId)>,
    history: Rc<RefCell<BrowserHistory>>,
    registerd_routes: RefCell<RouteSlot>,
    slots: Rc<RefCell<Vec<(ScopeId, String)>>>,
    root_found: Rc<Cell<bool>>,
    cur_root: RefCell<String>,
    listener: HistoryListener,
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

        let slots: Rc<RefCell<Vec<(ScopeId, String)>>> = Default::default();

        let _slots = slots.clone();

        let root_found = Rc::new(Cell::new(false));
        let regen = regen_route.clone();
        let _root_found = root_found.clone();
        let listener = history.listen(move || {
            _root_found.set(false);
            // checking if the route is valid is cheap, so we do it
            for (slot, _) in _slots.borrow_mut().iter().rev() {
                log::trace!("regenerating slot {:?}", slot);
                regen(*slot);
            }
        });

        Self {
            registerd_routes: RefCell::new(RouteSlot::Routes {
                partial: String::from("/"),
                total: String::from("/"),
                rest: Vec::new(),
            }),
            root_found,
            history: Rc::new(RefCell::new(history)),
            regen_route,
            slots,
            cur_root: RefCell::new(path.to_string()),
            listener,
        }
    }

    pub fn push_route(&self, route: &str) {
        self.history.borrow_mut().push(route);
    }

    pub fn register_total_route(&self, route: String, scope: ScopeId, fallback: bool) {
        self.slots.borrow_mut().push((scope, route));
    }

    pub fn should_render(&self, scope: ScopeId) -> bool {
        if self.root_found.get() {
            return false;
        }

        let location = self.history.borrow().location();
        let path = location.path();

        let roots = self.slots.borrow();

        let root = roots.iter().find(|(id, route)| id == &scope);

        // fallback logic
        match root {
            Some((_id, route)) => {
                if route == path {
                    self.root_found.set(true);
                    true
                } else {
                    if route == "" {
                        self.root_found.set(true);
                        true
                    } else {
                        false
                    }
                }
            }
            None => false,
        }
    }
}

pub struct RouterCfg {
    initial_route: String,
}

impl RouterCfg {
    pub fn new(initial_route: String) -> Self {
        Self { initial_route }
    }
}
