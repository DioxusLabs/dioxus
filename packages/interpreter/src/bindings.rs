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
    pub fn SaveTemplate(this: &Interpreter, nodes: Vec<Node>, name: &str);

    #[wasm_bindgen(method)]
    pub fn MountToRoot(this: &Interpreter);

    #[wasm_bindgen(method)]
    pub fn AssignId(this: &Interpreter, path: &[u8], id: u32);

    #[wasm_bindgen(method)]
    pub fn CreatePlaceholder(this: &Interpreter, id: u32);

    #[wasm_bindgen(method)]
    pub fn CreateTextNode(this: &Interpreter, value: JsValue, id: u32);

    #[wasm_bindgen(method)]
    pub fn HydrateText(this: &Interpreter, path: &[u8], value: &str, id: u32);

    #[wasm_bindgen(method)]
    pub fn LoadTemplate(this: &Interpreter, name: &str, index: u32, id: u32);

    #[wasm_bindgen(method)]
    pub fn ReplaceWith(this: &Interpreter, id: u32, m: u32);

    #[wasm_bindgen(method)]
    pub fn ReplacePlaceholder(this: &Interpreter, path: &[u8], m: u32);

    #[wasm_bindgen(method)]
    pub fn InsertAfter(this: &Interpreter, id: u32, n: u32);

    #[wasm_bindgen(method)]
    pub fn InsertBefore(this: &Interpreter, id: u32, n: u32);

    #[wasm_bindgen(method)]
    pub fn SetAttribute(this: &Interpreter, id: u32, name: &str, value: JsValue, ns: Option<&str>);

    #[wasm_bindgen(method)]
    pub fn SetBoolAttribute(this: &Interpreter, id: u32, name: &str, value: bool);

    #[wasm_bindgen(method)]
    pub fn SetText(this: &Interpreter, id: u32, text: JsValue);

    #[wasm_bindgen(method)]
    pub fn NewEventListener(
        this: &Interpreter,
        name: &str,
        id: u32,
        handler: &Function,
        bubbles: bool,
    );

    #[wasm_bindgen(method)]
    pub fn RemoveEventListener(this: &Interpreter, name: &str, id: u32);

    #[wasm_bindgen(method)]
    pub fn RemoveAttribute(this: &Interpreter, id: u32, field: &str, ns: Option<&str>);

    #[wasm_bindgen(method)]
    pub fn Remove(this: &Interpreter, id: u32);

    #[wasm_bindgen(method)]
    pub fn PushRoot(this: &Interpreter, id: u32);
}
