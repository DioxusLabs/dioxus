use dioxus_core::changelist::Edit;
use fxhash::FxHashMap;
use log::debug;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{window, Document, Element, Event, Node};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CacheId(u32);

#[derive(Debug)]
pub(crate) struct PatchMachine {
    container: Element,
    pub(crate) stack: Stack,
    temporaries: FxHashMap<u32, Node>,
    callback: Option<Closure<dyn FnMut(&Event)>>,
    document: Document,
    // templates: FxHashMap<CacheId, Node>,
}

#[derive(Debug, Default)]
pub struct Stack {
    list: Vec<Node>,
}

impl Stack {
    pub fn with_capacity(cap: usize) -> Self {
        Stack {
            list: Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, node: Node) {
        debug!("stack-push: {:?}", node);
        self.list.push(node);
    }

    pub fn pop(&mut self) -> Node {
        let res = self.list.pop().unwrap();
        debug!("stack-pop: {:?}", res);

        res
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn top(&self) -> &Node {
        log::info!(
            "Called top of stack with {} items remaining",
            self.list.len()
        );
        match self.list.last() {
            Some(a) => a,
            None => panic!("Called 'top' of an empty stack, make sure to push the root first"),
        }
    }
}

impl PatchMachine {
    pub fn new(container: Element) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        Self {
            // templates: Default::default(),
            container,
            stack: Stack::with_capacity(20),
            temporaries: Default::default(),
            callback: None,
            document,
        }
    }

    pub fn unmount(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
        // self.templates.clear();
    }

    pub fn start(&mut self) {
        if let Some(child) = self.container.first_child() {
            self.stack.push(child);
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
    }

    pub fn get_template(&self, id: CacheId) -> Option<&Node> {
        todo!()
        // self.templates.get(&id)
    }

    pub fn init_events_trampoline(&mut self, _trampoline: ()) {
        todo!("Event trampoline not a thing anymore")
        // pub fn init_events_trampoline(&mut self, mut trampoline: EventsTrampoline) {
        // self.callback = Some(Closure::wrap(Box::new(move |event: &web_sys::Event| {
        //     let target = event
        //         .target()
        //         .expect("missing target")
        //         .dyn_into::<Element>()
        //         .expect("not a valid element");
        //     let typ = event.type_();
        //     let a: u32 = target
        //         .get_attribute(&format!("dodrio-a-{}", typ))
        //         .and_then(|v| v.parse().ok())
        //         .unwrap_or_default();

        //     let b: u32 = target
        //         .get_attribute(&format!("dodrio-b-{}", typ))
        //         .and_then(|v| v.parse().ok())
        //         .unwrap_or_default();

        //     // get a and b from the target
        //     trampoline(event.clone(), a, b);
        // }) as Box<dyn FnMut(&Event)>));
    }

    pub fn handle_edit(&mut self, edit: &Edit) {
        match *edit {
            // 0
            Edit::SetText { text } => {
                //
                self.stack.top().set_text_content(Some(text))
            }

            // 1
            Edit::RemoveSelfAndNextSiblings {} => {
                let node = self.stack.pop();
                let mut sibling = node.next_sibling();

                while let Some(inner) = sibling {
                    let temp = inner.next_sibling();
                    if let Some(sibling) = inner.dyn_ref::<Element>() {
                        sibling.remove();
                    }
                    sibling = temp;
                }
                if let Some(node) = node.dyn_ref::<Element>() {
                    node.remove();
                }
            }

            // 2
            Edit::ReplaceWith => {
                let new_node = self.stack.pop();
                let old_node = self.stack.pop();

                if old_node.has_type::<Element>() {
                    old_node
                        .dyn_ref::<Element>()
                        .unwrap()
                        .replace_with_with_node_1(&new_node)
                        .unwrap();
                } else if old_node.has_type::<web_sys::CharacterData>() {
                    old_node
                        .dyn_ref::<web_sys::CharacterData>()
                        .unwrap()
                        .replace_with_with_node_1(&new_node)
                        .unwrap();
                } else if old_node.has_type::<web_sys::DocumentType>() {
                    old_node
                        .dyn_ref::<web_sys::DocumentType>()
                        .unwrap()
                        .replace_with_with_node_1(&new_node)
                        .unwrap();
                } else {
                    panic!("Cannot replace node: {:?}", old_node);
                }

                self.stack.push(new_node);
            }

            // 3
            Edit::SetAttribute { name, value } => {
                let node = self.stack.top();

                if let Some(node) = node.dyn_ref::<Element>() {
                    node.set_attribute(name, value).unwrap();

                    // Some attributes are "volatile" and don't work through `setAttribute`.
                    // TODO:
                    // if name == "value" {
                    //     node.set_value(value);
                    // }
                    // if name == "checked" {
                    //     node.set_checked(true);
                    // }
                    // if name == "selected" {
                    //     node.set_selected(true);
                    // }
                }
            }

            // 4
            Edit::RemoveAttribute { name } => {
                let node = self.stack.top();
                if let Some(node) = node.dyn_ref::<Element>() {
                    node.remove_attribute(name).unwrap();

                    // Some attributes are "volatile" and don't work through `removeAttribute`.
                    // TODO:
                    // if name == "value" {
                    //     node.set_value("");
                    // }
                    // if name == "checked" {
                    //     node.set_checked(false);
                    // }
                    // if name == "selected" {
                    //     node.set_selected(false);
                    // }
                }
            }

            // 5
            Edit::PushReverseChild { n } => {
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(children.length() - n - 1).unwrap();
                self.stack.push(child);
            }

            // 6
            Edit::PopPushChild { n } => {
                self.stack.pop();
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(n).unwrap();
                self.stack.push(child);
            }

            // 7
            Edit::Pop => {
                self.stack.pop();
            }

            // 8
            Edit::AppendChild => {
                let child = self.stack.pop();
                self.stack.top().append_child(&child).unwrap();
            }

            // 9
            Edit::CreateTextNode { text } => self.stack.push(
                self.document
                    .create_text_node(text)
                    .dyn_into::<Node>()
                    .unwrap(),
            ),

            // 10
            Edit::CreateElement { tag_name } => {
                let el = self
                    .document
                    .create_element(tag_name)
                    .unwrap()
                    .dyn_into::<Node>()
                    .unwrap();
                self.stack.push(el);
            }

            // 11
            Edit::NewEventListener { event_type, a, b } => {
                let el = self.stack.top();

                let el = el
                    .dyn_ref::<Element>()
                    .expect(&format!("not an element: {:?}", el));
                el.add_event_listener_with_callback(
                    event_type,
                    self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
                )
                .unwrap();
                debug!("adding attributes: {}, {}", a, b);
                el.set_attribute(&format!("dodrio-a-{}", event_type), &a.to_string())
                    .unwrap();
                el.set_attribute(&format!("dodrio-b-{}", event_type), &b.to_string())
                    .unwrap();
            }

            // 12
            Edit::UpdateEventListener { event_type, a, b } => {
                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    el.set_attribute(&format!("dodrio-a-{}", event_type), &a.to_string())
                        .unwrap();
                    el.set_attribute(&format!("dodrio-b-{}", event_type), &b.to_string())
                        .unwrap();
                }
            }

            // 13
            Edit::RemoveEventListener { event_type } => {
                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    el.remove_event_listener_with_callback(
                        event_type,
                        self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .unwrap();
                }
            }

            // 14
            Edit::CreateElementNs { tag_name, ns } => {
                let el = self
                    .document
                    .create_element_ns(Some(ns), tag_name)
                    .unwrap()
                    .dyn_into::<Node>()
                    .unwrap();
                self.stack.push(el);
            }

            // 15
            Edit::SaveChildrenToTemporaries {
                mut temp,
                start,
                end,
            } => {
                let parent = self.stack.top();
                let children = parent.child_nodes();
                for i in start..end {
                    self.temporaries.insert(temp, children.get(i).unwrap());
                    temp += 1;
                }
            }

            // 16
            Edit::PushChild { n } => {
                let parent = self.stack.top();
                let child = parent.child_nodes().get(n).unwrap();
                self.stack.push(child);
            }

            // 17
            Edit::PushTemporary { temp } => {
                let t = self.temporaries.get(&temp).unwrap().clone();
                self.stack.push(t);
            }

            // 18
            Edit::InsertBefore => {
                let before = self.stack.pop();
                let after = self.stack.pop();
                after
                    .parent_node()
                    .unwrap()
                    .insert_before(&before, Some(&after))
                    .unwrap();
                self.stack.push(before);
            }

            // 19
            Edit::PopPushReverseChild { n } => {
                self.stack.pop();
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(children.length() - n - 1).unwrap();
                self.stack.push(child);
            }

            // 20
            Edit::RemoveChild { n } => {
                let parent = self.stack.top();
                if let Some(child) = parent.child_nodes().get(n).unwrap().dyn_ref::<Element>() {
                    child.remove();
                }
            }

            // 21
            Edit::SetClass { class_name } => {
                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    el.set_class_name(class_name);
                }
            }
        }
    }

    // // 24
    // pub fn save_template(&mut self, id: CacheId) {
    //     let template = self.stack.top();
    //     let t = template.clone_node_with_deep(true).unwrap();
    //     // self.templates.insert(id, t);
    // }

    // // 25
    // pub fn push_template(&mut self, id: CacheId) {
    //     let template = self.get_template(id).unwrap();
    //     let t = template.clone_node_with_deep(true).unwrap();
    //     self.stack.push(t);
    // }

    // pub fn has_template(&self, id: CacheId) -> bool {
    //     todo!()
    //     // self.templates.contains_key(&id)
    // }
}
