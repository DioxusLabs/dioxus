#[cfg(feature = "webonly")]
use js_sys::Function;
#[cfg(feature = "webonly")]
use sledgehammer_bindgen::bindgen;
#[cfg(feature = "webonly")]
use web_sys::Node;

#[cfg(feature = "webonly")]
pub const SLEDGEHAMMER_JS: &str = GENERATED_JS;

#[cfg(feature = "webonly")]
#[bindgen(module)]
mod js {
    const JS_FILE: &str = "./src/common.js";
    const JS: &str = r#"
    class ListenerMap {
        constructor(root) {
            // bubbling events can listen at the root element
            this.global = {};
            // non bubbling events listen at the element the listener was created at
            this.local = {};
            this.root = null;
            this.handler = null;
        }

        create(event_name, element, bubbles) {
            if (bubbles) {
                if (this.global[event_name] === undefined) {
                    this.global[event_name] = {};
                    this.global[event_name].active = 1;
                    this.root.addEventListener(event_name, this.handler);
                } else {
                    this.global[event_name].active++;
                }
            }
            else {
                const id = element.getAttribute("data-dioxus-id");
                if (!this.local[id]) {
                    this.local[id] = {};
                }
                element.addEventListener(event_name, this.handler);
            }
        }

        remove(element, event_name, bubbles) {
            if (bubbles) {
                this.global[event_name].active--;
                if (this.global[event_name].active === 0) {
                    this.root.removeEventListener(event_name, this.global[event_name].callback);
                    delete this.global[event_name];
                }
            }
            else {
                const id = element.getAttribute("data-dioxus-id");
                delete this.local[id][event_name];
                if (this.local[id].length === 0) {
                    delete this.local[id];
                }
                element.removeEventListener(event_name, this.handler);
            }
        }

        removeAllNonBubbling(element) {
            const id = element.getAttribute("data-dioxus-id");
            delete this.local[id];
        }
    }
    function LoadChild(ptr, len) {
        // iterate through each number and get that child
        node = stack[stack.length - 1];
        ptr_end = ptr + len;
        for (; ptr < ptr_end; ptr++) {
            end = m.getUint8(ptr);
            for (node = node.firstChild; end > 0; end--) {
                node = node.nextSibling;
            }
        }
        return node;
    }
    const listeners = new ListenerMap();
    let nodes = [];
    let stack = [];
    let root;
    const templates = {};
    let node, els, end, ptr_end, k;
    export function save_template(nodes, tmpl_id) {
        templates[tmpl_id] = nodes;
    }
    export function hydrate(ids) {
        const hydrateNodes = document.querySelectorAll('[data-node-hydration]');
        for (let i = 0; i < hydrateNodes.length; i++) {
            const hydrateNode = hydrateNodes[i];
            const hydration = hydrateNode.getAttribute('data-node-hydration');
            const split = hydration.split(',');
            const id = ids[parseInt(split[0])];
            nodes[id] = hydrateNode;
            if (split.length > 1) {
                hydrateNode.listening = split.length - 1;
                hydrateNode.setAttribute('data-dioxus-id', id);
                for (let j = 1; j < split.length; j++) {
                    const listener = split[j];
                    const split2 = listener.split(':');
                    const event_name = split2[0];
                    const bubbles = split2[1] === '1';
                    listeners.create(event_name, hydrateNode, bubbles);
                }
            }
        }
        const treeWalker = document.createTreeWalker(
            document.body,
            NodeFilter.SHOW_COMMENT,
        );
        let currentNode = treeWalker.nextNode();
        while (currentNode) {
            const id = currentNode.textContent;
            const split = id.split('node-id');
            if (split.length > 1) {
                nodes[ids[parseInt(split[1])]] = currentNode.nextSibling;
            }
            currentNode = treeWalker.nextNode();
        }
    }
    export function get_node(id) {
        return nodes[id];
    }
    export function initialize(root, handler) {
        listeners.handler = handler;
        nodes = [root];
        stack = [root];
        listeners.root = root;
    }
    function AppendChildren(id, many){
        root = nodes[id];
        els = stack.splice(stack.length-many);
        for (k = 0; k < many; k++) {
            root.appendChild(els[k]);
        }
    }
    "#;

    extern "C" {
        #[wasm_bindgen]
        pub fn save_template(nodes: Vec<Node>, tmpl_id: u16);

        #[wasm_bindgen]
        pub fn hydrate(ids: Vec<u32>);

        #[wasm_bindgen]
        pub fn get_node(id: u32) -> Node;

        #[wasm_bindgen]
        pub fn initialize(root: Node, handler: &Function);
    }

    fn mount_to_root() {
        "{AppendChildren(root, stack.length-1);}"
    }
    fn push_root(root: u32) {
        "{stack.push(nodes[$root$]);}"
    }
    fn append_children(id: u32, many: u16) {
        "{AppendChildren($id$, $many$);}"
    }
    fn pop_root() {
        "{stack.pop();}"
    }
    fn replace_with(id: u32, n: u16) {
        "{root = nodes[$id$]; els = stack.splice(stack.length-$n$); if (root.listening) { listeners.removeAllNonBubbling(root); } root.replaceWith(...els);}"
    }
    fn insert_after(id: u32, n: u16) {
        "{nodes[$id$].after(...stack.splice(stack.length-$n$));}"
    }
    fn insert_before(id: u32, n: u16) {
        "{nodes[$id$].before(...stack.splice(stack.length-$n$));}"
    }
    fn remove(id: u32) {
        "{node = nodes[$id$]; if (node !== undefined) { if (node.listening) { listeners.removeAllNonBubbling(node); } node.remove(); }}"
    }
    fn create_raw_text(text: &str) {
        "{stack.push(document.createTextNode($text$));}"
    }
    fn create_text_node(text: &str, id: u32) {
        "{node = document.createTextNode($text$); nodes[$id$] = node; stack.push(node);}"
    }
    fn create_placeholder(id: u32) {
        "{node = document.createElement('pre'); node.hidden = true; stack.push(node); nodes[$id$] = node;}"
    }
    fn new_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        r#"node = nodes[id]; if(node.listening){node.listening += 1;}else{node.listening = 1;} node.setAttribute('data-dioxus-id', `\${id}`); listeners.create($event_name$, node, $bubbles$);"#
    }
    fn remove_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
        "{node = nodes[$id$]; node.listening -= 1; node.removeAttribute('data-dioxus-id'); listeners.remove(node, $event_name$, $bubbles$);}"
    }
    fn set_text(id: u32, text: &str) {
        "{nodes[$id$].textContent = $text$;}"
    }
    fn set_attribute(id: u32, field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{node = nodes[$id$]; setAttributeInner(node, $field$, $value$, $ns$);}"
    }
    fn remove_attribute(id: u32, field: &str<u8, attr>, ns: &str<u8, ns_cache>) {
        r#"{
            node = nodes[$id$];
            if (!ns) {
                switch (field) {
                    case "value":
                        node.value = "";
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
                node.style.removeProperty(name);
            } else {
                node.removeAttributeNS(ns, field);
            }
        }"#
    }
    fn assign_id(ptr: u32, len: u8, id: u32) {
        "{nodes[$id$] = LoadChild($ptr$, $len$);}"
    }
    fn hydrate_text(ptr: u32, len: u8, value: &str, id: u32) {
        r#"{
            node = LoadChild($ptr$, $len$);
            if (node.nodeType == Node.TEXT_NODE) {
                node.textContent = value;
            } else {
                let text = document.createTextNode(value);
                node.replaceWith(text);
                node = text;
            }
            nodes[$id$] = node;
        }"#
    }
    fn replace_placeholder(ptr: u32, len: u8, n: u16) {
        "{els = stack.splice(stack.length - $n$); node = LoadChild($ptr$, $len$); node.replaceWith(...els);}"
    }
    fn load_template(tmpl_id: u16, index: u16, id: u32) {
        "{node = templates[$tmpl_id$][$index$].cloneNode(true); nodes[$id$] = node; stack.push(node);}"
    }
}

#[cfg(feature = "binary-protocol")]
pub mod binary_protocol {
    use sledgehammer_bindgen::bindgen;
    pub const SLEDGEHAMMER_JS: &str = GENERATED_JS;

    #[bindgen]
    mod protocol_js {
        const JS_FILE: &str = "./src/interpreter.js";
        const JS_FILE: &str = "./src/common.js";

        fn mount_to_root() {
            "{AppendChildren(root, stack.length-1);}"
        }
        fn push_root(root: u32) {
            "{stack.push(nodes[$root$]);}"
        }
        fn append_children(id: u32, many: u16) {
            "{AppendChildren($id$, $many$);}"
        }
        fn append_children_to_top(many: u16) {
            "{
                root = stack[stack.length-many-1];
                els = stack.splice(stack.length-many);
                for (k = 0; k < many; k++) {
                    root.appendChild(els[k]);
                }
            }"
        }
        fn pop_root() {
            "{stack.pop();}"
        }
        fn replace_with(id: u32, n: u16) {
            "{root = nodes[$id$]; els = stack.splice(stack.length-$n$); if (root.listening) { listeners.removeAllNonBubbling(root); } root.replaceWith(...els);}"
        }
        fn insert_after(id: u32, n: u16) {
            "{nodes[$id$].after(...stack.splice(stack.length-$n$));}"
        }
        fn insert_before(id: u32, n: u16) {
            "{nodes[$id$].before(...stack.splice(stack.length-$n$));}"
        }
        fn remove(id: u32) {
            "{node = nodes[$id$]; if (node !== undefined) { if (node.listening) { listeners.removeAllNonBubbling(node); } node.remove(); }}"
        }
        fn create_raw_text(text: &str) {
            "{stack.push(document.createTextNode($text$));}"
        }
        fn create_text_node(text: &str, id: u32) {
            "{node = document.createTextNode($text$); nodes[$id$] = node; stack.push(node);}"
        }
        fn create_element(element: &'static str<u8, el>) {
            "{stack.push(document.createElement($element$))}"
        }
        fn create_element_ns(element: &'static str<u8, el>, ns: &'static str<u8, namespace>) {
            "{stack.push(document.createElementNS($ns$, $element$))}"
        }
        fn create_placeholder(id: u32) {
            "{node = document.createElement('pre'); node.hidden = true; stack.push(node); nodes[$id$] = node;}"
        }
        fn add_placeholder() {
            "{node = document.createElement('pre'); node.hidden = true; stack.push(node);}"
        }
        fn new_event_listener(event: &str<u8, evt>, id: u32, bubbles: u8) {
            r#"
            bubbles = bubbles == 1;
            node = nodes[id];
            if(node.listening){
                node.listening += 1;
            } else {
                node.listening = 1;
            }
            node.setAttribute('data-dioxus-id', `\${id}`);
            const event_name = $event$;

            // if this is a mounted listener, we send the event immediately
            if (event_name === "mounted") {
                window.ipc.postMessage(
                    window.interpreter.serializeIpcMessage("user_event", {
                        name: event_name,
                        element: id,
                        data: null,
                        bubbles,
                    })
                );
            } else {
                listeners.create(event_name, node, bubbles, (event) => {
                    handler(event, event_name, bubbles, config);
                });
            }"#
        }
        fn remove_event_listener(event_name: &str<u8, evt>, id: u32, bubbles: u8) {
            "{node = nodes[$id$]; node.listening -= 1; node.removeAttribute('data-dioxus-id'); listeners.remove(node, $event_name$, $bubbles$);}"
        }
        fn set_text(id: u32, text: &str) {
            "{nodes[$id$].textContent = $text$;}"
        }
        fn set_attribute(id: u32, field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
            "{node = nodes[$id$]; setAttributeInner(node, $field$, $value$, $ns$);}"
        }
        fn set_top_attribute(field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
            "{setAttributeInner(stack[stack.length-1], $field$, $value$, $ns$);}"
        }
        fn remove_attribute(id: u32, field: &str<u8, attr>, ns: &str<u8, ns_cache>) {
            r#"{
                node = nodes[$id$];
                if (!ns) {
                    switch (field) {
                        case "value":
                            node.value = "";
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
                    node.style.removeProperty(name);
                } else {
                    node.removeAttributeNS(ns, field);
                }
            }"#
        }
        fn assign_id(array: &[u8], id: u32) {
            "{nodes[$id$] = LoadChild($array$);}"
        }
        fn hydrate_text(array: &[u8], value: &str, id: u32) {
            r#"{
                node = LoadChild($array$);
                if (node.nodeType == Node.TEXT_NODE) {
                    node.textContent = value;
                } else {
                    let text = document.createTextNode(value);
                    node.replaceWith(text);
                    node = text;
                }
                nodes[$id$] = node;
            }"#
        }
        fn replace_placeholder(array: &[u8], n: u16) {
            "{els = stack.splice(stack.length - $n$); node = LoadChild($array$); node.replaceWith(...els);}"
        }
        fn load_template(tmpl_id: u16, index: u16, id: u32) {
            "{node = templates[$tmpl_id$][$index$].cloneNode(true); nodes[$id$] = node; stack.push(node);}"
        }
        fn add_templates(tmpl_id: u16, len: u16) {
            "{templates[$tmpl_id$] = stack.splice(stack.length-$len$);}"
        }
    }
}
