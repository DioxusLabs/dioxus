use crate::Routable;
use std::{cell::RefCell, rc::Rc};

pub struct RouterService<R: Routable> {
    pub(crate) regen_route: Rc<dyn Fn()>,
    pub(crate) pending_routes: RefCell<Vec<R>>,
}

impl<R: Routable> RouterService<R> {
    pub fn current_path(&self) -> &str {
        todo!()
    }
    pub fn push_route(&self, route: R) {
        self.pending_routes.borrow_mut().push(route);
        (self.regen_route)();
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
