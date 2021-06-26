use crate::innerlude::*;

pub struct DebugDom {
    counter: u32,
}
impl DebugDom {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}
impl RealDom for DebugDom {
    fn push_root(&mut self, root: RealDomNode) {}

    fn append_child(&mut self) {}

    fn replace_with(&mut self) {}

    fn remove(&mut self) {}

    fn remove_all_children(&mut self) {}

    fn create_text_node(&mut self, text: &str) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_element(&mut self, tag: &str) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_element_ns(&mut self, tag: &str, namespace: &str) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn new_event_listener(
        &mut self,
        event: &str,
        scope: ScopeIdx,
        element_id: usize,
        realnode: RealDomNode,
    ) {
    }

    fn remove_event_listener(&mut self, event: &str) {}

    fn set_text(&mut self, text: &str) {}

    fn set_attribute(&mut self, name: &str, value: &str, is_namespaced: bool) {}

    fn remove_attribute(&mut self, name: &str) {}

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}
