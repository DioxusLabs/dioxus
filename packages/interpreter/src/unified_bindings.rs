#[cfg(feature = "webonly")]
use web_sys::Node;

pub const SLEDGEHAMMER_JS: &str = GENERATED_JS;

#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    pub type BaseInterpreter;

    #[wasm_bindgen(method)]
    pub fn initialize(this: &BaseInterpreter, root: Node, handler: &js_sys::Function);

    #[wasm_bindgen(method, js_name = "saveTemplate")]
    pub fn save_template(this: &BaseInterpreter, nodes: Vec<Node>, tmpl_id: u16);

    #[wasm_bindgen(method)]
    pub fn hydrate(this: &BaseInterpreter, ids: Vec<u32>, under: Vec<Node>);

    #[wasm_bindgen(method, js_name = "getNode")]
    pub fn get_node(this: &BaseInterpreter, id: u32) -> Node;

    #[wasm_bindgen(method, js_name = "pushRoot")]
    pub fn push_root(this: &BaseInterpreter, node: Node);
}

// Note that this impl is for the sledgehammer interpreter to allow us dropping down to the base interpreter
// During hydration and initialization we need to the base interpreter methods
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

    fn mount_to_root() {
        "{this.appendChildren(this.root, this.stack.length-1);}"
    }
    fn push_root(root: u32) {
        "{this.pushRoot(this.nodes[$root$]);}"
    }
    fn append_children(id: u32, many: u16) {
        "{this.appendChildren($id$, $many$);}"
    }
    fn pop_root() {
        "{this.stack.pop();}"
    }
    fn replace_with(id: u32, n: u16) {
        "{const root = this.nodes[$id$]; let els = this.stack.splice(this.stack.length-$n$); if (root.listening) { this.removeAllNonBubblingListeners(root); } root.replaceWith(...els);}"
    }
    fn insert_after(id: u32, n: u16) {
        "{let node = this.nodes[$id$];node.after(...this.stack.splice(this.stack.length-$n$));}"
    }
    fn insert_before(id: u32, n: u16) {
        "{let node = this.nodes[$id$];node.before(...this.stack.splice(this.stack.length-$n$));}"
    }
    fn remove(id: u32) {
        "{let node = this.nodes[$id$]; if (node !== undefined) { if (node.listening) { this.removeAllNonBubblingListeners(node); } node.remove(); }}"
    }
    fn create_raw_text(text: &str) {
        "{this.stack.push(document.createTextNode($text$));}"
    }
    fn create_text_node(text: &str, id: u32) {
        "{let node = document.createTextNode($text$); this.nodes[$id$] = node; this.stack.push(node);}"
    }
    fn create_placeholder(id: u32) {
        "{let node = document.createComment('placeholder'); this.stack.push(node); this.nodes[$id$] = node;}"
    }
    fn new_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        r#"
            let node = this.nodes[id];
            if(node.listening){node.listening += 1;}else{node.listening = 1;}
            node.setAttribute('data-dioxus-id', `\${id}`);
            this.createListener($event_name$, node, $bubbles$);
        "#
    }
    fn remove_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        "{let node = this.nodes[$id$]; node.listening -= 1; node.removeAttribute('data-dioxus-id'); this.removeListener(node, $event_name$, $bubbles$);}"
    }
    fn set_text(id: u32, text: &str) {
        "{this.nodes[$id$].textContent = $text$;}"
    }
    fn set_attribute(id: u32, field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{let node = this.nodes[$id$]; this.setAttributeInner(node, $field$, $value$, $ns$);}"
    }
    fn remove_attribute(id: u32, field: &str<u8, attr>, ns: &str<u8, ns_cache>) {
        r#"{
            let node = this.nodes[$id$];
            if (!ns) {
                switch (field) {
                    case "value":
                        node.value = "";
                        node.removeAttribute("value");
                        break;
                    case "checked":
                        node.checked = false;
                        break;
                    case "selected":
                        node.selected = false;
                        break;
                    case "dangerous_inner_html":
                        node.innerHTML = "";
                        break;
                    default:
                        node.removeAttribute(field);
                        break;
                }
            } else if (ns == "style") {
                node.style.removeProperty(field);
            } else {
                node.removeAttributeNS(ns, field);
            }
        }"#
    }
    fn assign_id(ptr: u32, len: u8, id: u32) {
        "{this.nodes[$id$] = this.loadChild($ptr$, $len$);}"
    }
    fn hydrate_text(ptr: u32, len: u8, value: &str, id: u32) {
        r#"{
            let node = this.loadChild($ptr$, $len$);
            if (node.nodeType == node.TEXT_NODE) {
                node.textContent = value;
            } else {
                let text = document.createTextNode(value);
                node.replaceWith(text);
                node = text;
            }
            this.nodes[$id$] = node;
        }"#
    }
    fn replace_placeholder(ptr: u32, len: u8, n: u16) {
        "{let els = this.stack.splice(this.stack.length - $n$); let node = this.loadChild($ptr$, $len$); node.replaceWith(...els);}"
    }
    fn load_template(tmpl_id: u16, index: u16, id: u32) {
        "{let node = this.templates[$tmpl_id$][$index$].cloneNode(true); this.nodes[$id$] = node; this.stack.push(node);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn append_children_to_top(many: u16) {
        "{
        let root = this.stack[this.stack.length-many-1];
        let els = this.stack.splice(this.stack.length-many);
        for (let k = 0; k < many; k++) {
            root.appendChild(els[k]);
        }
        }"
    }

    #[cfg(feature = "binary-protocol")]
    fn set_top_attribute(field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{this.setAttributeInner(this.stack[this.stack.length-1], $field$, $value$, $ns$);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn add_placeholder() {
        "{let node = document.createComment('placeholder'); this.stack.push(node);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn create_element(element: &'static str<u8, el>) {
        "{this.stack.push(document.createElement($element$))}"
    }

    #[cfg(feature = "binary-protocol")]
    fn create_element_ns(element: &'static str<u8, el>, ns: &'static str<u8, namespace>) {
        "{this.stack.push(document.createElementNS($ns$, $element$))}"
    }

    #[cfg(feature = "binary-protocol")]
    fn add_templates(tmpl_id: u16, len: u16) {
        "{this.templates[$tmpl_id$] = this.stack.splice(this.stack.length-$len$);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn foreign_event_listener(event: &str<u8, evt>, id: u32, bubbles: u8) {
        r#"
    bubbles = bubbles == 1;
    let this_node = this.nodes[id];
    if(this_node.listening){
        this_node.listening += 1;
    } else {
        this_node.listening = 1;
    }
    this_node.setAttribute('data-dioxus-id', `\${id}`);
    const event_name = $event$;

    // if this is a mounted listener, we send the event immediately
    if (event_name === "mounted") {
        window.ipc.postMessage(
            this.serializeIpcMessage("user_event", {
                name: event_name,
                element: id,
                data: null,
                bubbles,
            })
        );
    } else {
        this.createListener(event_name, this_node, bubbles, (event) => {
            this.handler(event, event_name, bubbles);
        });
    }"#
    }

    /// Assign the ID
    #[cfg(feature = "binary-protocol")]
    fn assign_id_ref(array: &[u8], id: u32) {
        "{this.nodes[$id$] = this.loadChild($array$);}"
    }

    /// The coolest ID ever!
    #[cfg(feature = "binary-protocol")]
    fn hydrate_text_ref(array: &[u8], value: &str, id: u32) {
        r#"{
        let node = this.loadChild($array$);
        if (node.nodeType == node.TEXT_NODE) {
            node.textContent = value;
        } else {
            let text = document.createTextNode(value);
            node.replaceWith(text);
            node = text;
        }
        this.nodes[$id$] = node;
    }"#
    }

    #[cfg(feature = "binary-protocol")]
    fn replace_placeholder_ref(array: &[u8], n: u16) {
        "{let els = this.stack.splice(this.stack.length - $n$); let node = this.loadChild($array$); node.replaceWith(...els);}"
    }
}
