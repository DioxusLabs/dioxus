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
    pub fn save_template(
        this: &BaseInterpreter,
        /// `js_sys::Array` of root entries. Each entry is either a DOM
        /// `Node` (the cloneable root for non-Dynamic template roots) or
        /// `null` for a root-level `TemplateNode::Dynamic` (those are
        /// never cloned — `create_dynamic_node` is invoked directly by the
        /// core diff). Sending `null` keeps the root-index dense without
        /// allocating a never-used placeholder DOM node.
        nodes: js_sys::Array,
        tmpl_id: u16,
        /// Per-template slot paths: a `js_sys::Array` of `Uint8Array`, one
        /// entry per `Template::node_paths()` slot. Each path is
        /// `[root_idx, child0, child1, ...]`. JS groups them per-root and
        /// uses them in `loadChild` to reinterpret byte paths that target
        /// (or pass through) a Dynamic slot. The real cloned DOM no longer
        /// has empty-text markers at those positions.
        slot_paths: js_sys::Array,
    );

    #[wasm_bindgen(method, js_name = "getNode")]
    pub fn get_node(this: &BaseInterpreter, id: u32) -> Node;

    #[wasm_bindgen(method, js_name = "pushRoot")]
    pub fn push_root(this: &BaseInterpreter, node: Node);
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

    fn push_root(root: u32) {
        "{this.pushRoot(this.nodes[$root$]);}"
    }
    fn append_children(id: u32, many: u16) {
        "{this.appendChildren($id$, $many$);}"
    }
    fn replace_with(id: u32, n: u16) {
        "{this.replaceWithChunk($id$,$n$);}"
    }
    fn insert_after(id: u32, n: u16) {
        "{this.insertChunkAfter($id$,$n$);}"
    }
    fn insert_before(id: u32, n: u16) {
        "{this.insertChunkBefore($id$,$n$);}"
    }
    fn remove(id: u32) {
        "{this.removeNode($id$);}"
    }
    fn create_raw_text(text: &str) {
        "{this.stack.push(document.createTextNode($text$));}"
    }
    fn create_text_node(text: &str, id: u32) {
        "{let node = document.createTextNode($text$); this.nodes[$id$] = node; this.stack.push(node);}"
    }
    fn new_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        r#"
            const node = this.nodes[id];
            if(node.listening){node.listening += 1;}else{node.listening = 1;}
            node.setAttribute('data-dioxus-id', `\${id}`);
            this.createListener($event_name$, node, $bubbles$);
        "#
    }
    fn remove_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        "{let node = this.nodes[$id$]; node.listening -= 1; node.removeAttribute('data-dioxus-id'); this.removeListener(node, $event_name$, $bubbles$);}"
    }
    fn set_text(id: u32, text: &str) {
        "{this.setNodeText($id$,$text$);}"
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
        // Map the byte path to its live DOM node (or virtual slot anchor) and
        // record it in the node table. Attribute paths always terminate at a
        // real element; node-slot paths terminate at a pre-built virtual
        // sentinel from `prepareTemplateClone`. `loadChild` handles both.
        "{this.nodes[$id$] = this.loadChild($ptr$, $len$);}"
    }
    fn insert_children_at_path(id: u32, ptr: u32, len: u8, n: u16) {
        "{this.insertChildrenAtPath($id$,$ptr$,$len$,$n$);}"
    }
    fn load_template(tmpl_id: u16, index: u16, id: u32) {
        // Clone the saved template root and prepare it for slot-aware path
        // resolution. The template no longer carries empty-text baseline
        // markers for `Dynamic { .. }` positions; instead the saved root
        // carries a `__dxSlotMap` / `__dxSlotPaths` pair (see `saveTemplate`)
        // that `prepareTemplateClone` translates into a per-clone
        // `__dxSlotAnchors` map of pre-built virtual placeholder sentinels —
        // one per slot, chained right-to-left within each parent so
        // reverse-order `replace_placeholder` calls resolve to the correct
        // live position even when adjacent slots share an end-of-parent
        // anchor.
        "{let node = this.prepareTemplateClone($tmpl_id$, $index$); this.nodes[$id$] = node; this.stack.push(node);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn append_children_to_top(many: u16) {
        // Slot sentinels pushed by `add_placeholder` are recorded as local
        // template-slot positions on the parent (in template-tree order)
        // instead of being inserted as real DOM children. `applyChunk` then
        // skips them; only real nodes hit the DOM.
        "{let top = this.stack[this.stack.length-$many$-1]; let els = this.stack.splice(this.stack.length-$many$); this.recordTemplateSlots(top, els); this.applyChunk(els, top, null);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn set_top_attribute(field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{this.setAttributeInner(this.stack[this.stack.length-1], $field$, $value$, $ns$);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn add_placeholder() {
        // Push a slot-anchor sentinel onto the build stack — NOT a real DOM
        // node. `append_children_to_top` records its position relative to
        // template-tree order on the parent element; `add_templates`
        // finalizes per-root slot maps for `loadChild` consumption.
        "{this.stack.push({__dxTemplateSlot: true});}"
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
        // Save each root and compute per-root `__dxSlotMap` / `__dxSlotPaths`
        // from the local `__dxLocalSlots` annotations attached during
        // `recordTemplateSlots`. Roots that happen to be slot sentinels
        // themselves are stored as `null` — they are never cloned (the core
        // diff routes root-level Dynamic slots through `create_dynamic_node`
        // directly).
        "{this.templates[$tmpl_id$] = this.finalizeTemplateRoots(this.stack.splice(this.stack.length-$len$));}"
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
            this.sendSerializedEvent({
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
        "{this.nodes[$id$] = this.loadChildBytes($array$);}"
    }

    #[cfg(feature = "binary-protocol")]
    fn insert_children_at_path_ref(id: u32, array: &[u8], n: u16) {
        "{this.insertChildrenAtPathBytes($id$, $array$, $n$);}"
    }
}
