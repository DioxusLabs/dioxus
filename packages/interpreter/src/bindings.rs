#![allow(clippy::unused_unit, non_upper_case_globals)]

use dioxus_core::Edits;
use dioxus_html::event_bubbles;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Event, Node};

#[used]
static mut MSG_PTR: usize = 0;
#[used]
static mut MSG_PTR_PTR: *const usize = unsafe { &MSG_PTR } as *const usize;
#[used]
static mut STR_PTR: usize = 0;
#[used]
static mut STR_PTR_PTR: *const usize = unsafe { &STR_PTR } as *const usize;
#[used]
static mut STR_LEN: usize = 0;
#[used]
static mut STR_LEN_PTR: *const usize = unsafe { &STR_LEN } as *const usize;
static mut ID_SIZE: u8 = 1;

fn get_id_size() -> u8 {
    unsafe { ID_SIZE }
}
fn set_id_size(size: u8) {
    unsafe {
        ID_SIZE = size;
    }
}

#[wasm_bindgen(module = "/src/interpreter.js")]
extern "C" {
    fn work_last_created(mem: JsValue);

    pub type JsInterpreter;

    #[wasm_bindgen(constructor)]
    pub fn new(
        arg: Element,
        mem: JsValue,
        msg_ptr: usize,
        str_ptr: usize,
        str_len_ptr: usize,
    ) -> JsInterpreter;

    #[wasm_bindgen(method)]
    pub fn Work(this: &JsInterpreter, mem: JsValue);

    #[wasm_bindgen(method)]
    pub fn SetNode(this: &JsInterpreter, id: usize, node: Node);

    #[wasm_bindgen(method)]
    pub fn SetEventHandler(this: &JsInterpreter, handler: &Closure<dyn FnMut(&Event)>);
}

pub struct Interpreter {
    js_interpreter: JsInterpreter,
}

#[derive(Default)]
pub struct InterpreterEdits {
    msg: Vec<u8>,
    /// A separate buffer for batched string decoding to avoid js-native overhead
    str_buf: Vec<u8>,
}

impl<'a> Edits<'a> for InterpreterEdits {
    fn is_empty(&self) -> bool {
        self.msg.is_empty()
    }

    fn append_children(&mut self, root: Option<u64>, children: Vec<u64>) {
        let root = root.map(|id| self.check_id(id));
        for child in &children {
            self.check_id(*child);
        }
        self.msg.push(Op::AppendChildren as u8);
        self.encode_maybe_id(root);
        self.msg
            .extend_from_slice(&(children.len() as u32).to_le_bytes());
        for child in children {
            self.encode_id(child.to_le_bytes());
        }
    }

    fn replace_with(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        let root = root.map(|id| self.check_id(id));
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.push(Op::ReplaceWith as u8);
        self.encode_maybe_id(root);
        self.msg
            .extend_from_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    fn insert_after(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        let root = root.map(|id| self.check_id(id));
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.push(Op::InsertAfter as u8);
        self.encode_maybe_id(root);
        self.msg
            .extend_from_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    fn insert_before(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        let root = root.map(|id| self.check_id(id));
        for child in &nodes {
            self.check_id(*child);
        }
        self.msg.push(Op::InsertBefore as u8);
        self.encode_maybe_id(root);
        self.msg
            .extend_from_slice(&(nodes.len() as u32).to_le_bytes());
        for node in nodes {
            self.encode_id(node.to_le_bytes());
        }
    }

    fn remove(&mut self, id: Option<u64>) {
        let root = id.map(|id| self.check_id(id));
        self.msg.push(Op::Remove as u8);
        self.encode_maybe_id(root);
    }

    fn create_text_node(&mut self, text: &'a str, id: Option<u64>) {
        let root = id.map(|id| self.check_id(id));
        self.msg.push(Op::CreateTextNode as u8);
        self.encode_maybe_id(root);
        self.encode_str(text);
    }

    fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: Option<u64>,
        children: u32,
    ) {
        let root = id.map(|id| self.check_id(id));
        self.msg.push(Op::CreateElement as u8);
        self.encode_maybe_id(root);
        self.encode_cachable_str(tag);
        if let Some(ns) = ns {
            self.msg.push(1);
            self.encode_cachable_str(ns);
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&children.to_le_bytes());
    }

    fn create_placeholder(&mut self, id: Option<u64>) {
        let root = id.map(|id| self.check_id(id));
        self.msg.push(Op::CreatePlaceholder as u8);
        self.encode_maybe_id(root);
    }

    fn new_event_listener(
        &mut self,
        listener: &dioxus_core::Listener,
        _scope: dioxus_core::ScopeId,
    ) {
        let root = listener
            .mounted_node
            .get()
            .map(|id| self.check_id(id.0 as u64));
        let name = listener.event;
        let bubbles = event_bubbles(name);
        self.msg.push(Op::NewEventListener as u8);
        self.encode_maybe_id(root);
        self.encode_cachable_str(name);
        self.msg.push(if bubbles { 1 } else { 0 });
    }

    fn remove_event_listener(&mut self, event: &'static str, root: Option<u64>) {
        let name = event;
        let bubbles = event_bubbles(name);
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::RemoveEventListener as u8);
        self.encode_maybe_id(root);
        self.encode_cachable_str(name);
        self.msg.push(if bubbles { 1 } else { 0 });
    }

    fn set_text(&mut self, text: &'a str, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::SetText as u8);
        self.encode_maybe_id(root);
        self.encode_str(text);
    }

    fn set_attribute(&mut self, attribute: &'a dioxus_core::Attribute<'a>, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        let field = attribute.attribute.name;
        let ns = attribute.attribute.namespace;
        self.msg.push(Op::SetAttribute as u8);
        self.encode_maybe_id(root);
        self.encode_cachable_str(field);
        if let Some(ns) = ns {
            self.msg.push(1);
            self.encode_cachable_str(ns);
        } else {
            self.msg.push(0);
        }
        if let Some(s) = attribute.value.as_text() {
            self.encode_str(s);
        } else {
            self.encode_str(&attribute.value.to_string());
        }
    }

    fn remove_attribute(&mut self, attribute: &dioxus_core::Attribute, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        let field = attribute.attribute.name;
        let ns = attribute.attribute.namespace;
        self.msg.push(Op::RemoveAttribute as u8);
        self.encode_maybe_id(root);
        self.encode_cachable_str(field);
        if let Some(ns) = ns {
            self.msg.push(1);
            self.encode_cachable_str(ns);
        } else {
            self.msg.push(0);
        }
    }

    fn clone_node(&mut self, id: Option<u64>, new_id: u64) {
        let root = id.map(|id| self.check_id(id));
        self.msg.push(Op::CloneNode as u8);
        self.encode_maybe_id(root);
        self.msg.extend_from_slice(&new_id.to_le_bytes());
    }

    fn clone_node_children(&mut self, id: Option<u64>, new_ids: Vec<u64>) {
        let root = id.map(|id| self.check_id(id));
        for id in &new_ids {
            self.check_id(*id);
        }
        self.msg.push(Op::CloneNodeChildren as u8);
        self.encode_maybe_id(root);
        for id in new_ids {
            self.encode_maybe_id(Some(id.to_le_bytes()));
        }
    }

    fn first_child(&mut self) {
        self.msg.push(Op::FirstChild as u8);
    }

    fn next_sibling(&mut self) {
        self.msg.push(Op::NextSibling as u8);
    }

    fn parent_node(&mut self) {
        self.msg.push(Op::ParentNode as u8);
    }

    fn store_with_id(&mut self, id: u64) {
        let id = self.check_id(id);
        self.msg.push(Op::StoreWithId as u8);
        self.encode_id(id);
    }

    fn set_last_node(&mut self, id: u64) {
        let id = self.check_id(id);
        self.msg.push(Op::SetLastNode as u8);
        self.encode_id(id);
    }
}

#[allow(non_snake_case)]
impl Interpreter {
    pub fn new(arg: Element) -> Interpreter {
        format!(
            "init: {:?}, {:?}, {:?}",
            unsafe { MSG_PTR_PTR as usize },
            unsafe { STR_PTR_PTR as usize },
            unsafe { STR_LEN_PTR as usize }
        );
        let js_interpreter = unsafe {
            JsInterpreter::new(
                arg,
                wasm_bindgen::memory(),
                MSG_PTR_PTR as usize,
                STR_PTR_PTR as usize,
                STR_LEN_PTR as usize,
            )
        };
        Interpreter { js_interpreter }
    }

    #[inline]
    pub fn SetNode(&mut self, id: usize, node: Node) {
        self.js_interpreter.SetNode(id, node);
    }

    #[inline]
    pub fn apply_edits(&mut self, mut edits: InterpreterEdits) {
        assert_eq!(0usize.to_le_bytes().len(), 32 / 8);
        edits.msg.push(Op::Stop as u8);
        unsafe {
            let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_PTR_PTR);
            *mut_ptr_ptr = edits.msg.as_ptr() as usize;
            let mut_str_ptr_ptr: *mut usize = std::mem::transmute(STR_PTR_PTR);
            *mut_str_ptr_ptr = edits.str_buf.as_ptr() as usize;
            let mut_str_len_ptr: *mut usize = std::mem::transmute(STR_LEN_PTR);
            *mut_str_len_ptr = edits.str_buf.len() as usize;
        }
        work_last_created(wasm_bindgen::memory());
        edits.msg.clear();
        edits.str_buf.clear();
    }

    #[inline]
    pub fn set_event_handler(&self, handler: &Closure<dyn FnMut(&Event)>) {
        self.js_interpreter.SetEventHandler(handler);
    }
}

impl InterpreterEdits {
    #[inline]
    pub fn should_flush(&self) -> bool {
        // self.msg.len() > 16384
        false
    }

    #[inline]
    fn encode_maybe_id(&mut self, id: Option<[u8; 8]>) {
        match id {
            Some(id) => {
                self.msg.push(1);
                self.encode_id(id);
            }
            None => {
                self.msg.push(0);
            }
        }
    }

    #[inline]
    fn encode_id(&mut self, bytes: [u8; 8]) {
        self.msg
            .extend_from_slice(&bytes[..(get_id_size() as usize)]);
    }

    #[inline]
    fn check_id(&mut self, id: u64) -> [u8; 8] {
        let bytes = id.to_le_bytes();
        let first_contentful_byte = bytes.iter().rev().position(|&b| b != 0).unwrap_or(8);
        let byte_size = (8 - first_contentful_byte) as u8;
        if byte_size > get_id_size() {
            self.set_byte_size(byte_size);
        }
        bytes
    }

    #[inline]
    fn set_byte_size(&mut self, byte_size: u8) {
        set_id_size(byte_size);
        self.msg.push(Op::SetIdSize as u8);
        self.msg.push(byte_size);
    }

    fn encode_str(&mut self, string: &str) {
        self.msg
            .extend_from_slice(&(string.len() as u16).to_le_bytes());
        self.str_buf.extend_from_slice(string.as_bytes());
    }

    fn encode_cachable_str(&mut self, string: &str) {
        self.msg
            .extend_from_slice(&(string.len() as u16).to_le_bytes());
        self.str_buf.extend_from_slice(string.as_bytes());
    }
}

enum Op {
    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    // /// The parent to append nodes to.
    // root: Option<u64>,

    // /// The ids of the children to append.
    // children: Vec<u64>,
    AppendChildren = 0,

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    // /// The ID of the node to be replaced.
    // root: Option<u64>,

    // /// The ids of the nodes to replace the root with.
    // nodes: Vec<u64>,
    ReplaceWith = 1,

    /// Insert a number of nodes after a given node.
    // /// The ID of the node to insert after.
    // root: Option<u64>,

    // /// The ids of the nodes to insert after the target node.
    // nodes: Vec<u64>,
    InsertAfter = 2,

    /// Insert a number of nodes before a given node.
    // /// The ID of the node to insert before.
    // root: Option<u64>,

    // /// The ids of the nodes to insert before the target node.
    // nodes: Vec<u64>,
    InsertBefore = 3,

    /// Remove a particular node from the DOM
    // /// The ID of the node to remove.
    // root: Option<u64>,
    Remove = 4,

    /// Create a new purely-text node
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The textcontent of the node
    // text: &'bump str,
    CreateTextNode = 5,

    /// Create a new purely-element node
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The tagname of the node
    // tag: &'bump str,

    // /// The number of children nodes that will follow this message.
    // children: u32,
    /// Create a new purely-comment node with a given namespace
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The namespace of the node
    // tag: &'bump str,

    // /// The namespace of the node (like `SVG`)
    // ns: &'static str,

    // /// The number of children nodes that will follow this message.
    // children: u32,
    CreateElement = 6,

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node.
    // /// The ID the new node should have.
    // root: Option<u64>,
    CreatePlaceholder = 7,

    /// Create a new Event Listener.
    // /// The name of the event to listen for.
    // event_name: &'static str,

    // /// The ID of the node to attach the listener to.
    // scope: ScopeId,

    // /// The ID of the node to attach the listener to.
    // root: Option<u64>,
    NewEventListener = 8,

    /// Remove an existing Event Listener.
    // /// The ID of the node to remove.
    // root: Option<u64>,

    // /// The name of the event to remove.
    // event: &'static str,
    RemoveEventListener = 9,

    /// Set the textcontent of a node.
    // /// The ID of the node to set the textcontent of.
    // root: Option<u64>,

    // /// The textcontent of the node
    // text: &'bump str,
    SetText = 10,

    /// Set the value of a node's attribute.
    // /// The ID of the node to set the attribute of.
    // root: Option<u64>,

    // /// The name of the attribute to set.
    // field: &'static str,

    // /// The value of the attribute.
    // value: AttributeValue<'bump>,

    // // value: &'bump str,
    // /// The (optional) namespace of the attribute.
    // /// For instance, "style" is in the "style" namespace.
    // ns: Option<&'bump str>,
    SetAttribute = 11,

    /// Remove an attribute from a node.
    // /// The ID of the node to remove.
    // root: Option<u64>,

    // /// The name of the attribute to remove.
    // name: &'static str,

    // /// The namespace of the attribute.
    // ns: Option<&'bump str>,
    RemoveAttribute = 12,

    /// Clones a node.
    // /// The ID of the node to clone.
    // id: Option<u64>,

    // /// The ID of the new node.
    // new_id: u64,
    CloneNode = 13,

    /// Clones the children of a node. (allows cloning fragments)
    // /// The ID of the node to clone.
    // id: Option<u64>,

    // /// The ID of the new node.
    // new_ids: Vec<u64>,
    CloneNodeChildren = 14,

    /// Navigates to the last node to the first child of the current node.
    FirstChild = 15,

    /// Navigates to the last node to the last child of the current node.
    NextSibling = 16,

    /// Navigates to the last node to the parent of the current node.
    ParentNode = 17,

    /// Stores the last node with a new id.
    // /// The ID of the node to store.
    // id: u64,
    StoreWithId = 18,

    /// Manually set the last node.
    // /// The ID to set the last node to.
    // id: u64,
    SetLastNode = 19,

    /// Set id size
    SetIdSize = 20,

    /// Stop
    Stop = 21,
}
