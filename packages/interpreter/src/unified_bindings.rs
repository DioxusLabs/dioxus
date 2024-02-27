#[cfg(feature = "webonly")]
use js_sys::Function;

#[cfg(feature = "webonly")]
use web_sys::Node;

use sledgehammer_bindgen::bindgen;

pub const SLEDGEHAMMER_JS: &str = GENERATED_JS;

#[bindgen(module)]
mod js {
    // Load in the JavaScript file with all the imports
    const JS_FILE: &str = "./src/gen/interpreter.js";

    // Boot the interpreter and attach us some bindings!
    const JS: &str = r#""#;

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

    /*
    Binary protocol methods only!

    These methods let us support binary packing mutations for use on boundaries like desktop where we prefer to send
    binary data instead of JSON.

    We're using native types in a number of places
    */
    fn append_children_to_top(many: u16) {
        "{
            root = stack[stack.length-many-1];
            els = stack.splice(stack.length-many);
            for (k = 0; k < many; k++) {
                root.appendChild(els[k]);
            }
        }"
    }
    fn set_top_attribute(field: &str<u8, attr>, value: &str, ns: &str<u8, ns_cache>) {
        "{setAttributeInner(stack[stack.length-1], $field$, $value$, $ns$);}"
    }
    fn add_placeholder() {
        "{node = document.createElement('pre'); node.hidden = true; stack.push(node);}"
    }
    fn create_element_ns(element: &'static str<u8, el>, ns: &'static str<u8, namespace>) {
        "{stack.push(document.createElementNS($ns$, $element$))}"
    }
    fn create_element(element: &'static str<u8, el>) {
        "{stack.push(document.createElement($element$))}"
    }
    fn add_templates(tmpl_id: u16, len: u16) {
        "{templates[$tmpl_id$] = stack.splice(stack.length-$len$);}"
    }
    fn foreign_event_listener(event: &str<u8, evt>, id: u32, bubbles: u8) {
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
    fn assign_id_ref(array: &[u8], id: u32) {
        "{nodes[$id$] = LoadChild($array$);}"
    }
    fn hydrate_text_ref(array: &[u8], value: &str, id: u32) {
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

    fn replace_placeholder_ref(array: &[u8], n: u16) {
        "{els = stack.splice(stack.length - $n$); node = LoadChild($array$); node.replaceWith(...els);}"
    }
}
