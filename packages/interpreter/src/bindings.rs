#![allow(clippy::unused_unit, non_upper_case_globals)]

use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

#[wasm_bindgen(module = "/src/interpreter.js")]
extern "C" {
    pub type Interpreter;

    #[wasm_bindgen(constructor)]
    pub fn new(arg: Element) -> Interpreter;

    #[wasm_bindgen(method)]
    pub fn SetNode(this: &Interpreter, id: usize, node: Node);

    #[wasm_bindgen(method)]
    pub fn AppendChildren(this: &Interpreter, root: Option<u64>, children: Vec<u64>);

    #[wasm_bindgen(method)]
    pub fn ReplaceWith(this: &Interpreter, root: Option<u64>, nodes: Vec<u64>);

    #[wasm_bindgen(method)]
    pub fn InsertAfter(this: &Interpreter, root: Option<u64>, nodes: Vec<u64>);

    #[wasm_bindgen(method)]
    pub fn InsertBefore(this: &Interpreter, root: Option<u64>, nodes: Vec<u64>);

    #[wasm_bindgen(method)]
    pub fn Remove(this: &Interpreter, root: Option<u64>);

    #[wasm_bindgen(method)]
    pub fn CreateTextNode(this: &Interpreter, text: JsValue, root: Option<u64>);

    #[wasm_bindgen(method)]
    pub fn CreateElement(this: &Interpreter, tag: &str, root: Option<u64>, children: u32);

    #[wasm_bindgen(method)]
    pub fn CreateElementNs(
        this: &Interpreter,
        tag: &str,
        root: Option<u64>,
        ns: &str,
        children: u32,
    );

    #[wasm_bindgen(method)]
    pub fn CreatePlaceholder(this: &Interpreter, root: Option<u64>);

    #[wasm_bindgen(method)]
    pub fn NewEventListener(
        this: &Interpreter,
        name: &str,
        root: Option<u64>,
        handler: &Function,
        bubbles: bool,
    );

    #[wasm_bindgen(method)]
    pub fn RemoveEventListener(this: &Interpreter, root: Option<u64>, name: &str, bubbles: bool);

    #[wasm_bindgen(method)]
    pub fn SetText(this: &Interpreter, root: Option<u64>, text: JsValue);

    #[wasm_bindgen(method)]
    pub fn SetAttribute(
        this: &Interpreter,
        root: Option<u64>,
        field: &str,
        value: JsValue,
        ns: Option<&str>,
    );

    #[wasm_bindgen(method)]
    pub fn RemoveAttribute(this: &Interpreter, root: Option<u64>, field: &str, ns: Option<&str>);

    #[wasm_bindgen(method)]
    pub fn CloneNode(this: &Interpreter, root: Option<u64>, new_id: u64);

    #[wasm_bindgen(method)]
    pub fn CloneNodeChildren(this: &Interpreter, root: Option<u64>, new_ids: Vec<u64>);

    #[wasm_bindgen(method)]
    pub fn FirstChild(this: &Interpreter);

    #[wasm_bindgen(method)]
    pub fn NextSibling(this: &Interpreter);

    #[wasm_bindgen(method)]
    pub fn ParentNode(this: &Interpreter);

    #[wasm_bindgen(method)]
    pub fn StoreWithId(this: &Interpreter, id: u64);

    #[wasm_bindgen(method)]
    pub fn SetLastNode(this: &Interpreter, id: u64);
}
