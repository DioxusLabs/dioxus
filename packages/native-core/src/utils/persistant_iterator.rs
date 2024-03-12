use smallvec::SmallVec;

use crate::{
    node::FromAnyValue,
    node_watcher::NodeWatcher,
    prelude::{NodeMut, NodeRef},
    real_dom::{NodeImmutable, RealDom},
    NodeId,
};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

/// The element produced by the iterator
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ElementProduced {
    id: NodeId,
    movement: IteratorMovement,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// The method by which the iterator produced an element
pub enum IteratorMovement {
    /// The iterator produced an element by progressing to the next node
    Progressed,
    /// The iterator reached the end of the tree and looped back to the root
    Looped,
}

impl ElementProduced {
    /// Get the id of the element produced
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The movement the iterator made to produce the element
    pub fn movement(&self) -> &IteratorMovement {
        &self.movement
    }

    fn looped(id: NodeId) -> Self {
        Self {
            id,
            movement: IteratorMovement::Looped,
        }
    }

    fn progressed(id: NodeId) -> Self {
        Self {
            id,
            movement: IteratorMovement::Progressed,
        }
    }
}

struct PersistantElementIterUpdater<V> {
    stack: Arc<Mutex<smallvec::SmallVec<[NodeId; 5]>>>,
    phantom: std::marker::PhantomData<V>,
}

impl<V: FromAnyValue + Sync + Send> NodeWatcher<V> for PersistantElementIterUpdater<V> {
    fn on_node_moved(&mut self, node: NodeMut<V>) {
        // if any element is moved, update its parents in the stack
        let mut stack = self.stack.lock().unwrap();
        let moved = node.id();
        let rdom = node.real_dom();
        if let Some(r) = stack.iter().position(|el_id| *el_id == moved) {
            let back = &stack[r..];
            let mut new = SmallVec::new();
            let mut parent = node.parent_id();
            while let Some(p) = parent.and_then(|id| rdom.get(id)) {
                new.push(p.id());
                parent = p.parent_id();
            }
            new.extend(back.iter().copied());
            *stack = new;
        }
    }

    fn on_node_removed(&mut self, node: NodeMut<V>) {
        // if any element is removed in the chain, remove it and its children from the stack
        let mut stack = self.stack.lock().unwrap();
        let removed = node.id();
        if let Some(r) = stack.iter().position(|el_id| *el_id == removed) {
            stack.truncate(r);
        }
    }
}

/// Focus systems need a iterator that can persist through changes in the [crate::prelude::RealDom]
/// This iterator traverses the tree depth first.
/// You can iterate through it with [PersistantElementIter::next] and [PersistantElementIter::prev].
/// The iterator loops around when it reaches the end or the beginning.
pub struct PersistantElementIter {
    // stack of elements and fragments, the last element is the last element that was yielded
    stack: Arc<Mutex<smallvec::SmallVec<[NodeId; 5]>>>,
}

impl PersistantElementIter {
    /// Create a new iterator in the RealDom
    pub fn create<V: FromAnyValue + Send + Sync>(rdom: &mut RealDom<V>) -> Self {
        let inner = Arc::new(Mutex::new(smallvec::smallvec![rdom.root_id()]));

        rdom.add_node_watcher(PersistantElementIterUpdater {
            stack: inner.clone(),
            phantom: std::marker::PhantomData,
        });

        PersistantElementIter { stack: inner }
    }

    /// get the next element
    pub fn next<V: FromAnyValue + Send + Sync>(&mut self, rdom: &RealDom<V>) -> ElementProduced {
        let mut stack = self.stack.lock().unwrap();
        if stack.is_empty() {
            let id = rdom.root_id();
            let new = id;
            stack.push(new);
            ElementProduced::looped(id)
        } else {
            let mut look_in_children = true;
            loop {
                if let Some(current) = stack.last().and_then(|last| rdom.get(*last)) {
                    // if the current element has children, add the first child to the stack and return it
                    if look_in_children {
                        if let Some(first) = current.children().first() {
                            let new = first.id();
                            stack.push(new);
                            return ElementProduced::progressed(new);
                        }
                    }
                    stack.pop();
                    if let Some(new) = current.next() {
                        // the next element exists, add it to the stack and return it
                        let new = new.id();
                        stack.push(new);
                        return ElementProduced::progressed(new);
                    }
                    // otherwise, continue the loop and go to the parent
                } else {
                    // if there is no parent, loop back to the root
                    let new = rdom.root_id();
                    stack.clear();
                    stack.push(new);
                    return ElementProduced::looped(new);
                }
                look_in_children = false;
            }
        }
    }

    /// get the previous element
    pub fn prev<V: FromAnyValue + Send + Sync>(&mut self, rdom: &RealDom<V>) -> ElementProduced {
        // recursively add the last child element to the stack
        fn push_back<V: FromAnyValue + Send + Sync>(
            stack: &mut smallvec::SmallVec<[NodeId; 5]>,
            node: NodeRef<V>,
        ) -> NodeId {
            stack.push(node.id());
            if let Some(last) = node.children().last() {
                push_back(stack, *last)
            } else {
                node.id()
            }
        }
        let mut stack = self.stack.lock().unwrap();
        if stack.is_empty() {
            let id = rdom.root_id();
            let last = push_back(&mut stack, rdom.get(id).unwrap());
            ElementProduced::looped(last)
        } else if let Some(current) = stack.pop().and_then(|last| rdom.get(last)) {
            if let Some(new) = current.prev() {
                // the next element exists, add it to the stack and return it
                let new = push_back(&mut stack, new);
                ElementProduced::progressed(new)
            }
            // otherwise, yeild the parent
            else if let Some(parent) = stack.last() {
                // if there is a parent, return it
                ElementProduced::progressed(*parent)
            } else {
                // if there is no parent, loop back to the root
                let id = rdom.root_id();
                let last = push_back(&mut stack, rdom.get(id).unwrap());
                ElementProduced::looped(last)
            }
        } else {
            // if there is no parent, loop back to the root
            let id = rdom.root_id();
            let last = push_back(&mut stack, rdom.get(id).unwrap());
            ElementProduced::looped(last)
        }
    }
}

#[test]
#[allow(unused_variables)]
fn traverse() {
    use crate::dioxus::DioxusState;
    use crate::prelude::*;
    use dioxus::prelude::*;
    #[allow(non_snake_case)]
    fn Base() -> Element {
        rsx!(
            div{
                div{
                    "hello"
                    p{
                        "world"
                    }
                    "hello world"
                }
            }
        )
    }

    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom = RealDom::new([]);

    let mut iter = PersistantElementIter::create(&mut rdom);
    let mut dioxus_state = DioxusState::create(&mut rdom);
    vdom.rebuild(&mut dioxus_state.create_mutation_writer(&mut rdom));

    let div_tag = "div".to_string();
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));
    let text1 = "hello".to_string();
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text1)
    ));
    let p_tag = "p".to_string();
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: p_tag, .. })
    ));
    let text2 = "world".to_string();
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text2)
    ));
    let text3 = "hello world".to_string();
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text3)
    ));
    assert!(matches!(
        &*rdom.get(iter.next(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));

    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text3)
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text2)
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: p_tag, .. })
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text1)
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: div_tag, .. })
    ));
    assert!(matches!(
        &*rdom.get(iter.prev(&rdom).id()).unwrap().node_type(),
        NodeType::Text(text3)
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_removes() {
    use crate::dioxus::DioxusState;
    use crate::prelude::*;
    use dioxus::prelude::*;
    #[allow(non_snake_case)]
    fn Base() -> Element {
        let children = match generation() % 2 {
            0 => 3,
            1 => 2,
            _ => unreachable!(),
        };
        rsx!(
            div {
                for i in 0..children {
                    p { key: "{i}", "{i}" }
                }
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom = RealDom::new([]);

    // this will end on the node that is removed
    let mut iter1 = PersistantElementIter::create(&mut rdom);
    // this will end on the after node that is removed
    let mut iter2 = PersistantElementIter::create(&mut rdom);
    let mut dioxus_state = DioxusState::create(&mut rdom);

    vdom.rebuild(&mut dioxus_state.create_mutation_writer(&mut rdom));

    // root
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // div
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // "1"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // "2"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter2.next(&rdom).id();
    // "3"
    iter2.next(&rdom).id();

    vdom.mark_dirty(ScopeId::ROOT);
    vdom.render_immediate(&mut dioxus_state.create_mutation_writer(&mut rdom));

    let root_tag = "Root".to_string();
    let idx = iter1.next(&rdom).id();
    assert!(matches!(
        &*rdom.get(idx).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: root_tag, .. })
    ));

    let idx = iter2.next(&rdom).id();
    assert!(matches!(
        &*rdom.get(idx).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: root_tag, .. })
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_before() {
    use crate::dioxus::DioxusState;
    use crate::prelude::*;
    use dioxus::prelude::*;
    #[allow(non_snake_case)]
    fn Base() -> Element {
        let children = match generation() % 2 {
            0 => 3,
            1 => 2,
            _ => unreachable!(),
        };
        rsx!(
            div {
                for i in 0..children {
                    p { key: "{i}", "{i}" }
                }
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom = RealDom::new([]);
    let mut dioxus_state = DioxusState::create(&mut rdom);

    vdom.rebuild(&mut dioxus_state.create_mutation_writer(&mut rdom));

    let mut iter = PersistantElementIter::create(&mut rdom);
    // div
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "1"
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "2"
    iter.next(&rdom).id();

    vdom.mark_dirty(ScopeId::ROOT);
    vdom.render_immediate(&mut dioxus_state.create_mutation_writer(&mut rdom));

    let p_tag = "div".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &*rdom.get(idx).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: p_tag, .. })
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_after() {
    use crate::dioxus::DioxusState;
    use crate::prelude::*;
    use dioxus::prelude::*;
    #[allow(non_snake_case)]
    fn Base() -> Element {
        let children = match generation() % 2 {
            0 => 3,
            1 => 2,
            _ => unreachable!(),
        };
        rsx!(
            div{
                for i in 0..children {
                    p { key: "{i}", "{i}" }
                }
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom = RealDom::new([]);
    let mut iter = PersistantElementIter::create(&mut rdom);
    let mut dioxus_state = DioxusState::create(&mut rdom);

    let mut writer = dioxus_state.create_mutation_writer(&mut rdom);
    vdom.rebuild(&mut writer);

    // div
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "hello"
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "world"
    iter.next(&rdom).id();

    let mut writer = dioxus_state.create_mutation_writer(&mut rdom);
    vdom.rebuild(&mut writer);

    let p_tag = "p".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &*rdom.get(idx).unwrap().node_type(),
        NodeType::Element(ElementNode { tag: p_tag, .. })
    ));
    let text = "hello world".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &*rdom.get(idx).unwrap().node_type(),
        NodeType::Text(text)
    ));
}
