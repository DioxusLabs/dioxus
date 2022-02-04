use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

#[wasm_bindgen(module = "/src/interpreter.js")]
extern "C" {
    pub type Interpreter;

    #[wasm_bindgen(constructor)]
    pub fn new(arg: Element) -> Interpreter;

    #[wasm_bindgen(method)]
    pub fn set_node(this: &Interpreter, id: usize, node: Node);

    #[wasm_bindgen(method)]
    pub fn PushRoot(this: &Interpreter, root: u64);

    #[wasm_bindgen(method)]
    pub fn AppendChildren(this: &Interpreter, many: u32);

    #[wasm_bindgen(method)]
    pub fn ReplaceWith(this: &Interpreter, root: u64, m: u32);

    #[wasm_bindgen(method)]
    pub fn InsertAfter(this: &Interpreter, root: u64, n: u32);

    #[wasm_bindgen(method)]
    pub fn InsertBefore(this: &Interpreter, root: u64, n: u32);

    #[wasm_bindgen(method)]
    pub fn Remove(this: &Interpreter, root: u64);

    #[wasm_bindgen(method)]
    pub fn CreateTextNode(this: &Interpreter, text: &str, root: u64);

    #[wasm_bindgen(method)]
    pub fn CreateElement(this: &Interpreter, tag: &str, root: u64);

    #[wasm_bindgen(method)]
    pub fn CreateElementNs(this: &Interpreter, tag: &str, root: u64, ns: &str);

    #[wasm_bindgen(method)]
    pub fn CreatePlaceholder(this: &Interpreter, root: u64);

    #[wasm_bindgen(method)]
    pub fn NewEventListener(this: &Interpreter, name: &str, root: u64, handler: &Function);

    #[wasm_bindgen(method)]
    pub fn RemoveEventListener(this: &Interpreter, root: u64, name: &str);

    #[wasm_bindgen(method)]
    pub fn SetText(this: &Interpreter, root: u64, text: &str);

    #[wasm_bindgen(method)]
    pub fn SetAttribute(this: &Interpreter, root: u64, field: &str, value: &str, ns: Option<&str>);

    #[wasm_bindgen(method)]
    pub fn RemoveAttribute(this: &Interpreter, root: u64, field: &str);
}
