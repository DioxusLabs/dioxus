#![allow(clippy::unused_unit, non_upper_case_globals)]

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

#[wasm_bindgen(module = "/src/interpreter.js")]
extern "C" {
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
    msg: Vec<u8>,
    /// A separate buffer for batched string decoding to avoid js-native overhead
    str_buf: Vec<u8>,
    id_size: u8,
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
        Interpreter {
            js_interpreter,
            msg: Vec::new(),
            str_buf: Vec::new(),
            id_size: 1,
        }
    }

    pub fn SetNode(&mut self, id: usize, node: Node) {
        self.js_interpreter.SetNode(id, node);
    }

    pub fn AppendChildren(&mut self, root: Option<u64>, children: Vec<u64>) {
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

    pub fn ReplaceWith(&mut self, root: Option<u64>, nodes: Vec<u64>) {
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

    pub fn InsertAfter(&mut self, root: Option<u64>, nodes: Vec<u64>) {
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

    pub fn InsertBefore(&mut self, root: Option<u64>, nodes: Vec<u64>) {
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

    pub fn Remove(&mut self, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::Remove as u8);
        self.encode_maybe_id(root);
    }

    pub fn CreateTextNode(&mut self, text: &str, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::CreateTextNode as u8);
        self.encode_maybe_id(root);
        self.encode_str(text);
    }

    pub fn CreateElement(&mut self, tag: &str, root: Option<u64>, children: u32) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::CreateElement as u8);
        self.encode_maybe_id(root);
        self.encode_str(tag);
        self.msg.push(0);
        self.msg.extend_from_slice(&children.to_le_bytes());
    }

    pub fn CreateElementNs(&mut self, tag: &str, root: Option<u64>, ns: &str, children: u32) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::CreateElement as u8);
        self.encode_maybe_id(root);
        self.encode_str(tag);
        self.msg.push(1);
        self.encode_str(ns);
        self.msg.extend_from_slice(&children.to_le_bytes());
    }

    pub fn CreatePlaceholder(&mut self, root: Option<u64>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::CreatePlaceholder as u8);
        self.encode_maybe_id(root);
    }

    pub fn NewEventListener(&mut self, name: &str, root: Option<u64>, bubbles: bool) {
        let root = unsafe { root.unwrap_unchecked() };
        let root = self.check_id(root);
        self.msg.push(Op::NewEventListener as u8);
        self.encode_id(root);
        self.encode_str(name);
        self.msg.push(if bubbles { 1 } else { 0 });
    }

    pub fn RemoveEventListener(&mut self, root: Option<u64>, name: &str, bubbles: bool) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::RemoveEventListener as u8);
        self.encode_maybe_id(root);
        self.encode_str(name);
        self.msg.push(if bubbles { 1 } else { 0 });
    }

    pub fn SetText(&mut self, root: Option<u64>, text: &str) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::SetText as u8);
        self.encode_maybe_id(root);
        self.encode_str(text);
    }

    pub fn SetAttribute(&mut self, root: Option<u64>, field: &str, value: &str, ns: Option<&str>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::SetAttribute as u8);
        self.encode_maybe_id(root);
        self.encode_str(field);
        if let Some(ns) = ns {
            self.msg.push(1);
            self.encode_str(ns);
        } else {
            self.msg.push(0);
        }
        self.encode_str(value);
    }

    pub fn RemoveAttribute(&mut self, root: Option<u64>, field: &str, ns: Option<&str>) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::RemoveAttribute as u8);
        self.encode_maybe_id(root);
        self.encode_str(field);
        if let Some(ns) = ns {
            self.msg.push(1);
            self.encode_str(ns);
        } else {
            self.msg.push(0);
        }
    }

    pub fn CloneNode(&mut self, root: Option<u64>, new_id: u64) {
        let root = root.map(|id| self.check_id(id));
        self.msg.push(Op::CloneNode as u8);
        self.encode_maybe_id(root);
        self.msg.extend_from_slice(&new_id.to_le_bytes());
    }

    pub fn CloneNodeChildren(&mut self, root: Option<u64>, new_ids: Vec<u64>) {
        let root = root.map(|id| self.check_id(id));
        for id in &new_ids {
            self.check_id(*id);
        }
        self.msg.push(Op::CloneNodeChildren as u8);
        self.encode_maybe_id(root);
        for id in new_ids {
            self.encode_maybe_id(Some(id.to_le_bytes()));
        }
    }

    pub fn FirstChild(&mut self) {
        self.msg.push(Op::FirstChild as u8);
    }

    pub fn NextSibling(&mut self) {
        self.msg.push(Op::NextSibling as u8);
    }

    pub fn ParentNode(&mut self) {
        self.msg.push(Op::ParentNode as u8);
    }

    pub fn StoreWithId(&mut self, id: u64) {
        let id = self.check_id(id);
        self.msg.push(Op::StoreWithId as u8);
        self.encode_maybe_id(Some(id));
    }

    pub fn SetLastNode(&mut self, id: u64) {
        let id = self.check_id(id);
        self.msg.push(Op::SetLastNode as u8);
        self.encode_maybe_id(Some(id));
    }

    pub fn should_flush(&self) -> bool {
        self.msg.len() > 16384
    }

    pub fn flush(&mut self) {
        assert_eq!(0usize.to_le_bytes().len(), 32 / 8);
        self.msg.push(Op::Stop as u8);
        unsafe {
            let mut_ptr_ptr: *mut usize = std::mem::transmute(MSG_PTR_PTR);
            *mut_ptr_ptr = self.msg.as_ptr() as usize;
            let mut_str_ptr_ptr: *mut usize = std::mem::transmute(STR_PTR_PTR);
            *mut_str_ptr_ptr = self.str_buf.as_ptr() as usize;
            let mut_str_len_ptr: *mut usize = std::mem::transmute(STR_LEN_PTR);
            *mut_str_len_ptr = self.str_buf.len() as usize;
        }
        self.js_interpreter.Work(wasm_bindgen::memory());
        self.msg.clear();
        self.str_buf.clear();
    }

    pub fn set_event_handler(&self, handler: &Closure<dyn FnMut(&Event)>) {
        self.js_interpreter.SetEventHandler(handler);
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
            .extend_from_slice(&bytes[..(self.id_size as usize)]);
    }

    #[inline]
    fn check_id(&mut self, id: u64) -> [u8; 8] {
        let bytes = id.to_le_bytes();
        let first_contentful_byte = bytes.iter().rev().position(|&b| b != 0).unwrap_or(8);
        let byte_size = (8 - first_contentful_byte) as u8;
        if byte_size > self.id_size {
            self.set_byte_size(byte_size);
        }
        bytes
    }

    fn set_byte_size(&mut self, byte_size: u8) {
        self.id_size = byte_size;
        self.msg.push(Op::SetIdSize as u8);
        self.msg.push(byte_size);
    }

    fn encode_str(&mut self, string: &str) {
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
