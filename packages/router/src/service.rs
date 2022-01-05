use gloo::history::{BrowserHistory, History, HistoryListener};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus_core::ScopeId;

pub struct RouterService {
    pub(crate) regen_route: Rc<dyn Fn(ScopeId)>,
    history: RefCell<BrowserHistory>,
    registerd_routes: RefCell<RouteSlot>,
    slots: RefCell<HashMap<ScopeId, String>>,
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

        let regen = Rc::clone(&regen_route);
        let listener = history.listen(move || {
            regen(root_scope);
        });

        Self {
            registerd_routes: RefCell::new(RouteSlot::Routes {
                partial: String::from("/"),
                total: String::from("/"),
                rest: Vec::new(),
            }),
            history: RefCell::new(history),
            regen_route,
            slots: Default::default(),
            cur_root: RefCell::new(path.to_string()),
            listener,
        }
    }

    pub fn push_route(&self, route: &str) {
        self.history.borrow_mut().push(route);
    }

    pub fn register_total_route(&self, route: String, scope: ScopeId) {
        self.slots.borrow_mut().insert(scope, route);
    }

    pub fn should_render(&self, scope: ScopeId) -> bool {
        let location = self.history.borrow().location();
        let path = location.path();

        let roots = self.slots.borrow();

        let root = roots.get(&scope);

        match root {
            Some(r) => r == path,
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
