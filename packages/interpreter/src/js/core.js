// src/ts/set_attribute.ts
function setAttributeInner(node, field, value, ns) {
  if (ns === "style") {
    node.style.setProperty(field, value);
    return;
  }
  if (!!ns) {
    node.setAttributeNS(ns, field, value);
    return;
  }
  switch (field) {
    case "value":
      if (node.value !== value) {
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
      if (!truthy(value) && isBoolAttr(field)) {
        node.removeAttribute(field);
      } else {
        node.setAttribute(field, value);
      }
  }
}
var truthy = function(val) {
  return val === "true" || val === true;
};
var isBoolAttr = function(field) {
  switch (field) {
    case "allowfullscreen":
    case "allowpaymentrequest":
    case "async":
    case "autofocus":
    case "autoplay":
    case "checked":
    case "controls":
    case "default":
    case "defer":
    case "disabled":
    case "formnovalidate":
    case "hidden":
    case "ismap":
    case "itemscope":
    case "loop":
    case "multiple":
    case "muted":
    case "nomodule":
    case "novalidate":
    case "open":
    case "playsinline":
    case "readonly":
    case "required":
    case "reversed":
    case "selected":
    case "truespeed":
    case "webkitdirectory":
      return true;
    default:
      return false;
  }
};

// src/ts/core.ts
class BaseInterpreter {
  global;
  local;
  root;
  handler;
  nodes;
  stack;
  templates;
  m;
  constructor() {
  }
  initialize(root, handler = null) {
    this.global = {};
    this.local = {};
    this.root = root;
    this.nodes = [root];
    this.stack = [root];
    this.templates = {};
    if (handler) {
      this.handler = handler;
    }
  }
  createListener(event_name, element, bubbles) {
    if (bubbles) {
      if (this.global[event_name] === undefined) {
        this.global[event_name] = { active: 1, callback: this.handler };
        this.root.addEventListener(event_name, this.handler);
      } else {
        this.global[event_name].active++;
      }
    } else {
      const id = element.getAttribute("data-dioxus-id");
      if (!this.local[id]) {
        this.local[id] = {};
      }
      element.addEventListener(event_name, this.handler);
    }
  }
  removeListener(element, event_name, bubbles) {
    if (bubbles) {
      this.removeBubblingListener(event_name);
    } else {
      this.removeNonBubblingListener(element, event_name);
    }
  }
  removeBubblingListener(event_name) {
    this.global[event_name].active--;
    if (this.global[event_name].active === 0) {
      this.root.removeEventListener(event_name, this.global[event_name].callback);
      delete this.global[event_name];
    }
  }
  removeNonBubblingListener(element, event_name) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id][event_name];
    if (Object.keys(this.local[id]).length === 0) {
      delete this.local[id];
    }
    element.removeEventListener(event_name, this.handler);
  }
  removeAllNonBubblingListeners(element) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }
  getNode(id) {
    return this.nodes[id];
  }
  appendChildren(id, many) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0;k < many; k++) {
      root.appendChild(els[k]);
    }
  }
  loadChild(ptr, len) {
    let node = this.stack[this.stack.length - 1];
    let ptr_end = ptr + len;
    for (;ptr < ptr_end; ptr++) {
      let end = this.m.getUint8(ptr);
      for (node = node.firstChild;end > 0; end--) {
        node = node.nextSibling;
      }
    }
    return node;
  }
  saveTemplate(nodes, tmpl_id) {
    this.templates[tmpl_id] = nodes;
  }
  hydrateRoot(ids) {
    const hydrateNodes = document.querySelectorAll("[data-node-hydration]");
    for (let i = 0;i < hydrateNodes.length; i++) {
      const hydrateNode = hydrateNodes[i];
      const hydration = hydrateNode.getAttribute("data-node-hydration");
      const split = hydration.split(",");
      const id = ids[parseInt(split[0])];
      this.nodes[id] = hydrateNode;
      if (split.length > 1) {
        hydrateNode.listening = split.length - 1;
        hydrateNode.setAttribute("data-dioxus-id", id.toString());
        for (let j = 1;j < split.length; j++) {
          const listener = split[j];
          const split2 = listener.split(":");
          const event_name = split2[0];
          const bubbles = split2[1] === "1";
          this.createListener(event_name, hydrateNode, bubbles);
        }
      }
    }
    const treeWalker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
    let currentNode = treeWalker.nextNode();
    while (currentNode) {
      const id = currentNode.textContent;
      const split = id.split("node-id");
      if (split.length > 1) {
        this.nodes[ids[parseInt(split[1])]] = currentNode.nextSibling;
      }
      currentNode = treeWalker.nextNode();
    }
  }
  setAttributeInner(node, field, value, ns) {
    setAttributeInner(node, field, value, ns);
  }
}
export {
  BaseInterpreter
};