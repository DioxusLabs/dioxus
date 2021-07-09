use std::{cell::RefCell, rc::Rc};

use crate::innerlude::*;

// We actually allocate the properties for components in their parent's properties
// We then expose a handle to use those props for render in the form of "OpaqueComponent"
pub type OpaqueComponent = dyn for<'b> Fn(&'b Scope) -> VNode<'b>;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct EventQueue(pub Rc<RefCell<Vec<HeightMarker>>>);

impl EventQueue {
    pub fn new_channel(&self, height: u32, idx: ScopeIdx) -> Rc<dyn Fn()> {
        let inner = self.clone();
        let marker = HeightMarker { height, idx };
        Rc::new(move || {
            log::debug!("channel updated {:#?}", marker);
            inner.0.as_ref().borrow_mut().push(marker)
        })
    }
}

/// A helper type that lets scopes be ordered by their height
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeightMarker {
    pub idx: ScopeIdx,
    pub height: u32,
}

impl Ord for HeightMarker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl PartialOrd for HeightMarker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct DebugDom {
    counter: u64,
}
impl DebugDom {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}
impl<'a> RealDom<'a> for DebugDom {
    fn push_root(&mut self, root: RealDomNode) {}

    fn append_child(&mut self) {}

    fn replace_with(&mut self) {}

    fn remove(&mut self) {}

    fn remove_all_children(&mut self) {}

    fn create_text_node(&mut self, text: &str) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_element(&mut self, tag: &str, ns: Option<&'a str>) -> RealDomNode {
        self.counter += 1;
        RealDomNode::new(self.counter)
    }

    fn create_placeholder(&mut self) -> RealDomNode {
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

    fn set_attribute(&mut self, name: &str, value: &str, namespace: Option<&str>) {}

    fn remove_attribute(&mut self, name: &str) {}

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}
