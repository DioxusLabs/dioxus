#![allow(clippy::unused_unit, non_upper_case_globals)]

use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

#[used]
static mut PTR: usize = 0;
#[used]
static mut PTR_PTR: *const usize = unsafe { &PTR } as *const usize;
#[used]
static mut LEN_PTR: usize = 0;
#[used]
static mut LEN_PTR_PTR: *const usize = unsafe { &LEN_PTR } as *const usize;

#[wasm_bindgen(module = "/src/interpreter.js")]
extern "C" {
    pub type JsInterpreter;

    #[wasm_bindgen(constructor)]
    pub fn new(arg: Element, mem: JsValue, ptr: usize, size: usize) -> JsInterpreter;

    #[wasm_bindgen(method)]
    pub fn Work(this: &JsInterpreter);

    #[wasm_bindgen(method)]
    pub fn SetNode(this: &JsInterpreter, id: usize, node: Node);

    #[wasm_bindgen(method)]
    pub fn SetEventHandler(this: &JsInterpreter, handler: &Function);
}

pub struct Interpreter {
    js_interpreter: JsInterpreter,
    msg: Vec<u8>,
}

#[allow(non_snake_case)]
impl Interpreter {
    pub fn new(arg: Element) -> Interpreter {
        format!("init: {:?}, {:?}", unsafe { PTR_PTR as usize }, unsafe {
            LEN_PTR_PTR as usize
        });
        let js_interpreter = unsafe {
            JsInterpreter::new(
                arg,
                wasm_bindgen::memory(),
                PTR_PTR as usize,
                LEN_PTR_PTR as usize,
            )
        };
        Interpreter {
            js_interpreter,
            msg: Vec::new(),
        }
    }

    pub fn SetNode(&mut self, id: usize, node: Node) {
        self.js_interpreter.SetNode(id, node);
    }

    pub fn AppendChildren(&mut self, root: Option<u64>, children: Vec<u64>) {
        self.msg.push(Op::AppendChildren as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg
            .extend_from_slice(&(children.len() as u64).to_le_bytes());
        for child in children {
            self.msg.extend_from_slice(&(child + 1).to_le_bytes());
        }
    }

    pub fn ReplaceWith(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.msg.push(Op::ReplaceWith as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg
            .extend_from_slice(&(nodes.len() as u64).to_le_bytes());
        for node in nodes {
            self.msg.extend_from_slice(&(node + 1).to_le_bytes());
        }
    }

    pub fn InsertAfter(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.msg.push(Op::InsertAfter as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg
            .extend_from_slice(&(nodes.len() as u64).to_le_bytes());
        for node in nodes {
            self.msg.extend_from_slice(&(node + 1).to_le_bytes());
        }
    }

    pub fn InsertBefore(&mut self, root: Option<u64>, nodes: Vec<u64>) {
        self.msg.push(Op::InsertBefore as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg
            .extend_from_slice(&(nodes.len() as u64).to_le_bytes());
        for node in nodes {
            self.msg.extend_from_slice(&(node + 1).to_le_bytes());
        }
    }

    pub fn Remove(&mut self, root: Option<u64>) {
        self.msg.push(Op::Remove as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
    }

    pub fn CreateTextNode(&mut self, text: &str, root: Option<u64>) {
        self.msg.push(Op::CreateTextNode as u8);
        self.msg.extend_from_slice(&text.len().to_le_bytes());
        self.msg.extend_from_slice(text.as_bytes());
        self.msg.push(0);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
    }

    pub fn CreateElement(&mut self, tag: &str, root: Option<u64>, children: u32) {
        self.msg.push(Op::CreateElement as u8);
        self.msg.extend_from_slice(&tag.len().to_le_bytes());
        self.msg.extend_from_slice(tag.as_bytes());
        self.msg.push(0);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&children.to_le_bytes());
    }

    pub fn CreateElementNs(&mut self, tag: &str, root: Option<u64>, ns: &str, children: u32) {
        self.msg.push(Op::CreateElementNs as u8);
        self.msg.extend_from_slice(&tag.len().to_le_bytes());
        self.msg.extend_from_slice(tag.as_bytes());
        self.msg.push(0);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(ns.as_bytes());
        self.msg.push(0);
        self.msg.extend_from_slice(&children.to_le_bytes());
    }

    pub fn CreatePlaceholder(&mut self, root: Option<u64>) {
        self.msg.push(Op::CreatePlaceholder as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
    }

    pub fn NewEventListener(&mut self, name: &str, root: Option<u64>, bubbles: bool) {
        self.msg.push(Op::NewEventListener as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&name.len().to_le_bytes());
        self.msg.extend_from_slice(name.as_bytes());
        self.msg.push(bubbles as u8);
    }

    pub fn RemoveEventListener(&mut self, root: Option<u64>, name: &str, bubbles: bool) {
        self.msg.push(Op::RemoveEventListener as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&name.len().to_le_bytes());
        self.msg.extend_from_slice(name.as_bytes());
        self.msg.push(bubbles as u8);
    }

    pub fn SetText(&mut self, root: Option<u64>, text: &str) {
        self.msg.push(Op::SetText as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&text.len().to_le_bytes());
        self.msg.extend_from_slice(text.as_bytes());
    }

    pub fn SetAttribute(&mut self, root: Option<u64>, field: &str, value: &str, ns: Option<&str>) {
        self.msg.push(Op::SetAttribute as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&field.len().to_le_bytes());
        self.msg.extend_from_slice(field.as_bytes());
        self.msg.extend_from_slice(&value.len().to_le_bytes());
        self.msg.extend_from_slice(value.as_bytes());
        if let Some(ns) = ns {
            self.msg.extend_from_slice(&ns.len().to_le_bytes());
            self.msg.extend_from_slice(ns.as_bytes());
        } else {
            self.msg.push(0);
        }
    }

    pub fn RemoveAttribute(&mut self, root: Option<u64>, field: &str, ns: Option<&str>) {
        self.msg.push(Op::RemoveAttribute as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&field.len().to_le_bytes());
        self.msg.extend_from_slice(field.as_bytes());
        if let Some(ns) = ns {
            self.msg.extend_from_slice(&ns.len().to_le_bytes());
            self.msg.extend_from_slice(ns.as_bytes());
        } else {
            self.msg.push(0);
        }
    }

    pub fn CloneNode(&mut self, root: Option<u64>, new_id: u64) {
        self.msg.push(Op::CloneNode as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&(new_id + 1).to_le_bytes());
    }

    pub fn CloneNodeChildren(&mut self, root: Option<u64>, new_ids: Vec<u64>) {
        self.msg.push(Op::CloneNodeChildren as u8);
        if let Some(id) = root {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
        } else {
            self.msg.push(0);
        }
        self.msg.extend_from_slice(&new_ids.len().to_le_bytes());
        for id in new_ids {
            self.msg.extend_from_slice(&(id + 1).to_le_bytes());
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
        self.msg.push(Op::StoreWithId as u8);
        self.msg.extend_from_slice(&(id + 1).to_le_bytes());
    }

    pub fn SetLastNode(&mut self, id: u64) {
        self.msg.push(Op::SetLastNode as u8);
        self.msg.extend_from_slice(&(id + 1).to_le_bytes());
    }

    pub fn flush(&mut self) {
        let ptr = self.msg.as_ptr();
        let len = self.msg.len();
        unsafe {
            let mut_ptr_ptr: *mut usize = std::mem::transmute(PTR_PTR);
            *mut_ptr_ptr = ptr as usize;
            let mut_len_ptr_ptr: *mut usize = std::mem::transmute(LEN_PTR_PTR);
            *mut_len_ptr_ptr = len as usize;
        }
        self.js_interpreter.Work();
        self.msg.clear();
    }

    pub fn set_event_handler(&self, handler: &Function) {
        self.js_interpreter.SetEventHandler(handler);
    }
}

enum Op {
    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    // /// The parent to append nodes to.
    // root: Option<u64>,

    // /// The ids of the children to append.
    // children: Vec<u64>,
    AppendChildren,

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    // /// The ID of the node to be replaced.
    // root: Option<u64>,

    // /// The ids of the nodes to replace the root with.
    // nodes: Vec<u64>,
    ReplaceWith,

    /// Insert a number of nodes after a given node.
    // /// The ID of the node to insert after.
    // root: Option<u64>,

    // /// The ids of the nodes to insert after the target node.
    // nodes: Vec<u64>,
    InsertAfter,

    /// Insert a number of nodes before a given node.
    // /// The ID of the node to insert before.
    // root: Option<u64>,

    // /// The ids of the nodes to insert before the target node.
    // nodes: Vec<u64>,
    InsertBefore,

    /// Remove a particular node from the DOM
    // /// The ID of the node to remove.
    // root: Option<u64>,
    Remove,

    /// Create a new purely-text node
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The textcontent of the node
    // text: &'bump str,
    CreateTextNode,

    /// Create a new purely-element node
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The tagname of the node
    // tag: &'bump str,

    // /// The number of children nodes that will follow this message.
    // children: u32,
    CreateElement,

    /// Create a new purely-comment node with a given namespace
    // /// The ID the new node should have.
    // root: Option<u64>,

    // /// The namespace of the node
    // tag: &'bump str,

    // /// The namespace of the node (like `SVG`)
    // ns: &'static str,

    // /// The number of children nodes that will follow this message.
    // children: u32,
    CreateElementNs,

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node.
    // /// The ID the new node should have.
    // root: Option<u64>,
    CreatePlaceholder,

    /// Create a new Event Listener.
    // /// The name of the event to listen for.
    // event_name: &'static str,

    // /// The ID of the node to attach the listener to.
    // scope: ScopeId,

    // /// The ID of the node to attach the listener to.
    // root: Option<u64>,
    NewEventListener,

    /// Remove an existing Event Listener.
    // /// The ID of the node to remove.
    // root: Option<u64>,

    // /// The name of the event to remove.
    // event: &'static str,
    RemoveEventListener,

    /// Set the textcontent of a node.
    // /// The ID of the node to set the textcontent of.
    // root: Option<u64>,

    // /// The textcontent of the node
    // text: &'bump str,
    SetText,

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
    SetAttribute,

    /// Remove an attribute from a node.
    // /// The ID of the node to remove.
    // root: Option<u64>,

    // /// The name of the attribute to remove.
    // name: &'static str,

    // /// The namespace of the attribute.
    // ns: Option<&'bump str>,
    RemoveAttribute,

    /// Clones a node.
    // /// The ID of the node to clone.
    // id: Option<u64>,

    // /// The ID of the new node.
    // new_id: u64,
    CloneNode,

    /// Clones the children of a node. (allows cloning fragments)
    // /// The ID of the node to clone.
    // id: Option<u64>,

    // /// The ID of the new node.
    // new_ids: Vec<u64>,
    CloneNodeChildren,

    /// Navigates to the last node to the first child of the current node.
    FirstChild,

    /// Navigates to the last node to the last child of the current node.
    NextSibling,

    /// Navigates to the last node to the parent of the current node.
    ParentNode,

    /// Stores the last node with a new id.
    // /// The ID of the node to store.
    // id: u64,
    StoreWithId,

    /// Manually set the last node.
    // /// The ID to set the last node to.
    // id: u64,
    SetLastNode,
}
