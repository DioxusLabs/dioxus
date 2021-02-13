// use crate::cached_set::CacheId;
// use crate::{Element, EventsTrampoline};
use fxhash::FxHashMap;
use log::{debug, info, log};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{window, Document, Element, Event, Node};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) struct CacheId(u32);

#[derive(Debug)]
pub(crate) struct ChangeListInterpreter {
    container: Element,
    stack: Stack,
    temporaries: FxHashMap<u32, Node>,
    templates: FxHashMap<CacheId, Node>,
    callback: Option<Closure<dyn FnMut(&Event)>>,
    document: Document,
}

#[derive(Debug, Default)]
struct Stack {
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
        &self.list[self.list.len() - 1]
    }
}

impl ChangeListInterpreter {
    pub fn new(container: Element) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        Self {
            container,
            stack: Stack::with_capacity(20),
            temporaries: Default::default(),
            templates: Default::default(),
            callback: None,
            document,
        }
    }

    pub fn unmount(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
        self.templates.clear();
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
        self.templates.get(&id)
    }

    pub fn init_events_trampoline(&mut self, mut trampoline: ()) {
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

    // 0
    pub fn set_text(&mut self, text: &str) {
        self.stack.top().set_text_content(Some(text));
    }

    // 1
    pub fn remove_self_and_next_siblings(&mut self) {
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
    pub fn replace_with(&mut self) {
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
    pub fn set_attribute(&mut self, name: &str, value: &str) {
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
    pub fn remove_attribute(&mut self, name: &str) {
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
    pub fn push_reverse_child(&mut self, n: u32) {
        let parent = self.stack.top();
        let children = parent.child_nodes();
        let child = children.get(children.length() - n - 1).unwrap();
        self.stack.push(child);
    }

    // 6
    pub fn pop_push_child(&mut self, n: u32) {
        self.stack.pop();
        let parent = self.stack.top();
        let children = parent.child_nodes();
        let child = children.get(n).unwrap();
        self.stack.push(child);
    }

    // 7
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    // 8
    pub fn append_child(&mut self) {
        let child = self.stack.pop();
        self.stack.top().append_child(&child).unwrap();
    }

    // 9
    pub fn create_text_node(&mut self, text: &str) {
        self.stack.push(
            self.document
                .create_text_node(text)
                .dyn_into::<Node>()
                .unwrap(),
        );
    }

    // 10
    pub fn create_element(&mut self, tag_name: &str) {
        let el = self
            .document
            .create_element(tag_name)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();
        self.stack.push(el);
    }

    // 11
    pub fn new_event_listener(&mut self, event_type: &str, a: u32, b: u32) {
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
    pub fn update_event_listener(&mut self, event_type: &str, a: u32, b: u32) {
        if let Some(el) = self.stack.top().dyn_ref::<Element>() {
            el.set_attribute(&format!("dodrio-a-{}", event_type), &a.to_string())
                .unwrap();
            el.set_attribute(&format!("dodrio-b-{}", event_type), &b.to_string())
                .unwrap();
        }
    }

    // 13
    pub fn remove_event_listener(&mut self, event_type: &str) {
        if let Some(el) = self.stack.top().dyn_ref::<Element>() {
            el.remove_event_listener_with_callback(
                event_type,
                self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
            )
            .unwrap();
        }
    }

    // 16
    pub fn create_element_ns(&mut self, tag_name: &str, ns: &str) {
        let el = self
            .document
            .create_element_ns(Some(ns), tag_name)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();
        self.stack.push(el);
    }

    // 17
    pub fn save_children_to_temporaries(&mut self, mut temp: u32, start: u32, end: u32) {
        let parent = self.stack.top();
        let children = parent.child_nodes();
        for i in start..end {
            self.temporaries.insert(temp, children.get(i).unwrap());
            temp += 1;
        }
    }

    // 18
    pub fn push_child(&mut self, n: u32) {
        let parent = self.stack.top();
        let child = parent.child_nodes().get(n).unwrap();
        self.stack.push(child);
    }

    // 19
    pub fn push_temporary(&mut self, temp: u32) {
        let t = self.temporaries.get(&temp).unwrap().clone();
        self.stack.push(t);
    }

    // 20
    pub fn insert_before(&mut self) {
        let before = self.stack.pop();
        let after = self.stack.pop();
        after
            .parent_node()
            .unwrap()
            .insert_before(&before, Some(&after))
            .unwrap();
        self.stack.push(before);
    }

    // 21
    pub fn pop_push_reverse_child(&mut self, n: u32) {
        self.stack.pop();
        let parent = self.stack.top();
        let children = parent.child_nodes();
        let child = children.get(children.length() - n - 1).unwrap();
        self.stack.push(child);
    }

    // 22
    pub fn remove_child(&mut self, n: u32) {
        let parent = self.stack.top();
        if let Some(child) = parent.child_nodes().get(n).unwrap().dyn_ref::<Element>() {
            child.remove();
        }
    }

    // 23
    pub fn set_class(&mut self, class_name: &str) {
        if let Some(el) = self.stack.top().dyn_ref::<Element>() {
            el.set_class_name(class_name);
        }
    }

    // 24
    pub fn save_template(&mut self, id: CacheId) {
        let template = self.stack.top();
        let t = template.clone_node_with_deep(true).unwrap();
        self.templates.insert(id, t);
    }

    // 25
    pub fn push_template(&mut self, id: CacheId) {
        let template = self.get_template(id).unwrap();
        let t = template.clone_node_with_deep(true).unwrap();
        self.stack.push(t);
    }

    pub fn has_template(&self, id: CacheId) -> bool {
        self.templates.contains_key(&id)
    }
}
