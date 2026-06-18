#[cfg(feature = "webonly")]
use web_sys::Node;

pub const SLEDGEHAMMER_JS: &str = GENERATED_JS;

#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    pub type BaseInterpreter;

    #[wasm_bindgen(method)]
    pub fn initialize(this: &BaseInterpreter, root: Node, handler: &js_sys::Function);

    #[wasm_bindgen(method, js_name = "getNode")]
    pub fn get_node(this: &BaseInterpreter, id: u32) -> Node;

    #[wasm_bindgen(method, js_name = "pushRoot")]
    pub fn push_root(this: &BaseInterpreter, node: Node);

    /// Bind an ElementId to a DOM node (used by the hydration cursor).
    #[wasm_bindgen(method, js_name = "setNode")]
    pub fn set_node(this: &BaseInterpreter, id: u32, node: &Node);

    /// Attach an event listener to a previously-bound node by id.
    #[wasm_bindgen(method, js_name = "setNodeListener")]
    pub fn set_node_listener(this: &BaseInterpreter, id: u32, name: &str, bubbles: bool);
}

// Note that this impl is for the sledgehammer interpreter to allow us to
// access base interpreter methods from web setup and external helper modules.
#[cfg(feature = "webonly")]
impl Interpreter {
    /// Convert the interpreter to its baseclass, giving
    pub fn base(&self) -> &BaseInterpreter {
        use wasm_bindgen::prelude::JsCast;
        self.js_channel().unchecked_ref()
    }
}

#[sledgehammer_bindgen::bindgen(module)]
mod js {
    // Extend the web base class
    const BASE: &str = "./src/js/core.js";

    /// The interpreter extends the core interpreter which contains the state for the interpreter along with some functions that all platforms use like `AppendChildren`.
    #[extends(BaseInterpreter)]
    pub struct Interpreter;

    fn push_id(root: u32) {
        "{this.pushId($root$);}"
    }
    fn pop_id(id: u32) {
        "{this.popId($id$);}"
    }
    fn child(index: u32) {
        "{this.child($index$);}"
    }
    fn pop() {
        "{this.pop();}"
    }
    fn create_element_top(tag: &str<u8, el>, ns: &str<u8, namespace>) {
        "{this.createElementTop($tag$, $ns$ || null);}"
    }
    fn create_text(text: &str) {
        "{this.createTextTop($text$);}"
    }
    fn clone_node() {
        "{this.cloneTop();}"
    }
    fn append_children_top(many: u16) {
        "{this.appendChildrenToTop($many$);}"
    }
    fn replace_top_with(n: u16) {
        "{this.replaceTopWith($n$);}"
    }
    fn insert_after_top(n: u16) {
        "{this.insertAfterTop($n$);}"
    }
    fn insert_before_top(n: u16) {
        "{this.insertBeforeTop($n$);}"
    }
    fn remove_top() {
        "{this.removeTop();}"
    }
    fn set_top_text(text: &str) {
        "{this.setTextTop($text$);}"
    }
    fn set_current_attribute(field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{this.setTopAttribute($field$, $value$, $ns$ || null);}"
    }
    fn remove_current_attribute(field: &str<u8, attr>, ns: &str<u8, ns_cache>) {
        "{this.removeTopAttribute($field$, $ns$ || null);}"
    }
    fn new_top_event_listener(event_name: &str<u8, evt>, bubbles: u8) {
        "{this.addTopEventListener($event_name$, $bubbles$ === 1);}"
    }
    fn foreign_top_event_listener(event_name: &str<u8, evt>, bubbles: u8) {
        "{this.addTopForeignEventListener($event_name$, $bubbles$ === 1);}"
    }
    fn remove_top_event_listener(event_name: &str<u8, evt>, bubbles: u8) {
        "{this.removeTopEventListener($event_name$, $bubbles$ === 1);}"
    }
}
