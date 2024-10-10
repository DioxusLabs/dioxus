use std::rc::Rc;

use crate::History;

/// A [`History`] provider that stores all navigation information in memory.
pub struct LensHistory {
    parent_provider: Rc<dyn History>,
    // Take a parent route and return a child route or none if the route is not part of the child
    parent_to_child_route: fn(String) -> Option<String>,
    // Take a child route and return a parent route
    child_to_parent_route: fn(String) -> String,
}

impl LensHistory {
    pub fn new(
        parent_provider: Rc<dyn History>,
        parent_to_child_route: fn(String) -> Option<String>,
        child_to_parent_route: fn(String) -> String,
    ) -> Self {
        Self {
            parent_provider,
            parent_to_child_route,
            child_to_parent_route,
        }
    }
}

impl History for LensHistory {
    fn full_route_path(&self) -> String {
        self.parent_provider.full_route_path()
    }

    fn current_route(&self) -> String {
        let parent_current_route = self.parent_provider.current_route();
        (self.parent_to_child_route)(parent_current_route).unwrap_or_else(|| "/".to_string())
    }

    fn can_go_back(&self) -> bool {
        self.parent_provider.can_go_back()
    }

    fn go_back(&self) {
        self.parent_provider.go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.parent_provider.can_go_forward()
    }

    fn go_forward(&self) {
        self.parent_provider.go_forward()
    }

    fn push(&self, new: String) {
        let parent_route = (self.child_to_parent_route)(new.clone());
        self.parent_provider.push(parent_route);
    }

    fn replace(&self, path: String) {
        let parent_route = (self.child_to_parent_route)(path.clone());
        self.parent_provider.replace(parent_route);
    }
}
