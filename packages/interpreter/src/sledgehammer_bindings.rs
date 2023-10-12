use js_sys::Function;
use sledgehammer_bindgen::bindgen;
use web_sys::Node;

#[bindgen]
mod js {
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
    function SetAttributeInner(node, field, value, ns) {
        const name = field;
        if (ns === "style") {
            // ????? why do we need to do this
            if (node.style === undefined) {
                node.style = {};
            }
            node.style[name] = value;
        } else if (ns !== null && ns !== undefined && ns !== "") {
            node.setAttributeNS(ns, name, value);
        } else {
            switch (name) {
                case "value":
                    if (value !== node.value) {
                        node.value = value;
                    }
                    break;
                case "initial_value":
                    node.defaultValue = value;
                    break;
                case "checked":
                    node.checked = truthy(value);
                    break;
                case "initial_checked":
                    node.defaultChecked = truthy(value);
                    break;
                case "selected":
                    node.selected = truthy(value);
                    break;
                case "initial_selected":
                    node.defaultSelected = truthy(value);
                    break;
                case "dangerous_inner_html":
                    node.innerHTML = value;
                    break;
                default:
                    // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
                    if (!truthy(value) && bool_attrs.hasOwnProperty(name)) {
                        node.removeAttribute(name);
                    } else {
                        node.setAttribute(name, value);
                    }
            }
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
    export function set_node(id, node) {
        nodes[id] = node;
    }
    export function get_node(id) {
        return nodes[id];
    }
    export function initilize(root, handler) {
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
    const bool_attrs = {
        allowfullscreen: true,
        allowpaymentrequest: true,
        async: true,
        autofocus: true,
        autoplay: true,
        checked: true,
        controls: true,
        default: true,
        defer: true,
        disabled: true,
        formnovalidate: true,
        hidden: true,
        ismap: true,
        itemscope: true,
        loop: true,
        multiple: true,
        muted: true,
        nomodule: true,
        novalidate: true,
        open: true,
        playsinline: true,
        readonly: true,
        required: true,
        reversed: true,
        selected: true,
        truespeed: true,
        webkitdirectory: true,
      };
      function truthy(val) {
        return val === "true" || val === true;
      }
    "#;

    extern "C" {
        #[wasm_bindgen]
        pub fn save_template(nodes: Vec<Node>, tmpl_id: u32);

        #[wasm_bindgen]
        pub fn set_node(id: u32, node: Node);

        #[wasm_bindgen]
        pub fn get_node(id: u32) -> Node;

        #[wasm_bindgen]
        pub fn initilize(root: Node, handler: &Function);
    }

    fn mount_to_root() {
        "{AppendChildren(root, stack.length-1);}"
    }
    fn push_root(root: u32) {
        "{stack.push(nodes[$root$]);}"
    }
    fn append_children(id: u32, many: u32) {
        "{AppendChildren($id$, $many$);}"
    }
    fn pop_root() {
        "{stack.pop();}"
    }
    fn replace_with(id: u32, n: u32) {
        "{root = nodes[$id$]; els = stack.splice(stack.length-$n$); if (root.listening) { listeners.removeAllNonBubbling(root); } root.replaceWith(...els);}"
    }
    fn insert_after(id: u32, n: u32) {
        "{nodes[$id$].after(...stack.splice(stack.length-$n$));}"
    }
    fn insert_before(id: u32, n: u32) {
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
        "{node = nodes[$id$]; SetAttributeInner(node, $field$, $value$, $ns$);}"
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
    fn replace_placeholder(ptr: u32, len: u8, n: u32) {
        "{els = stack.splice(stack.length - $n$); node = LoadChild($ptr$, $len$); node.replaceWith(...els);}"
    }
    fn load_template(tmpl_id: u32, index: u32, id: u32) {
        "{node = templates[$tmpl_id$][$index$].cloneNode(true); nodes[$id$] = node; stack.push(node);}"
    }
}
