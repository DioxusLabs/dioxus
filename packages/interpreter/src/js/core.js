function setAttributeInner(node, field, value, ns) {
  if (ns === "style") {
    node.style.setProperty(field, value);
    return;
  }
  if (ns) {
    node.setAttributeNS(ns, field, value);
    return;
  }
  switch (field) {
    case "value":
      if (node.value !== value) node.value = value;
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
      if (!truthy(value) && isBoolAttr(field)) node.removeAttribute(field);
      else node.setAttribute(field, value);
  }
}
function truthy(val) {
  return val === "true" || val === !0;
}
function isBoolAttr(field) {
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
      return !0;
    default:
      return !1;
  }
}
class BaseInterpreter {
  global;
  local;
  root;
  handler;
  resizeObserver;
  intersectionObserver;
  nodes;
  stack;
  templates;
  m;
  constructor() {}
  initialize(root, handler = null) {
    (this.global = {}),
      (this.local = {}),
      (this.root = root),
      (this.nodes = [root]),
      (this.stack = [root]),
      (this.templates = {}),
      (this.handler = handler),
      root.setAttribute("data-dioxus-id", "0");
  }
  handleResizeEvent(entry) {
    let target = entry.target,
      event = new CustomEvent("resize", { bubbles: !1, detail: entry });
    target.dispatchEvent(event);
  }
  createResizeObserver(element) {
    if (!this.resizeObserver)
      this.resizeObserver = new ResizeObserver((entries) => {
        for (let entry of entries) this.handleResizeEvent(entry);
      });
    this.resizeObserver.observe(element);
  }
  removeResizeObserver(element) {
    if (this.resizeObserver) this.resizeObserver.unobserve(element);
  }
  handleIntersectionEvent(entry) {
    let target = entry.target,
      event = new CustomEvent("visible", { bubbles: !1, detail: entry });
    target.dispatchEvent(event);
  }
  createIntersectionObserver(element) {
    if (!this.intersectionObserver)
      this.intersectionObserver = new IntersectionObserver((entries) => {
        for (let entry of entries) this.handleIntersectionEvent(entry);
      });
    this.intersectionObserver.observe(element);
  }
  removeIntersectionObserver(element) {
    if (this.intersectionObserver) this.intersectionObserver.unobserve(element);
  }
  createListener(event_name, element, bubbles) {
    if (event_name == "resize") this.createResizeObserver(element);
    else if (event_name == "visible") this.createIntersectionObserver(element);
    if (bubbles)
      if (this.global[event_name] === void 0)
        (this.global[event_name] = { active: 1, callback: this.handler }),
          this.root.addEventListener(event_name, this.handler);
      else this.global[event_name].active++;
    else {
      let id = element.getAttribute("data-dioxus-id");
      if (!this.local[id]) this.local[id] = {};
      element.addEventListener(event_name, this.handler);
    }
  }
  removeListener(element, event_name, bubbles) {
    if (event_name == "resize") this.removeResizeObserver(element);
    else if (event_name == "visible") this.removeIntersectionObserver(element);
    else if (bubbles) this.removeBubblingListener(event_name);
    else this.removeNonBubblingListener(element, event_name);
  }
  removeBubblingListener(event_name) {
    if (
      (this.global[event_name].active--, this.global[event_name].active === 0)
    )
      this.root.removeEventListener(
        event_name,
        this.global[event_name].callback
      ),
        delete this.global[event_name];
  }
  removeNonBubblingListener(element, event_name) {
    let id = element.getAttribute("data-dioxus-id");
    if (
      (delete this.local[id][event_name],
      Object.keys(this.local[id]).length === 0)
    )
      delete this.local[id];
    element.removeEventListener(event_name, this.handler);
  }
  removeAllNonBubblingListeners(element) {
    let id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }
  getNode(id) {
    return this.nodes[id];
  }
  pushRoot(node) {
    this.stack.push(node);
  }
  appendChildren(id, many) {
    let root = this.nodes[id],
      els = this.stack.splice(this.stack.length - many);
    for (let k = 0; k < many; k++) root.appendChild(els[k]);
  }
  loadChild(ptr, len) {
    let node = this.stack[this.stack.length - 1],
      ptr_end = ptr + len;
    for (; ptr < ptr_end; ptr++) {
      let end = this.m.getUint8(ptr);
      for (node = node.firstChild; end > 0; end--) node = node.nextSibling;
    }
    return node;
  }
  saveTemplate(nodes, tmpl_id) {
    this.templates[tmpl_id] = nodes;
  }
  hydrate_node(hydrateNode, ids) {
    let split = hydrateNode.getAttribute("data-node-hydration").split(","),
      id = ids[parseInt(split[0])];
    if (((this.nodes[id] = hydrateNode), split.length > 1)) {
      (hydrateNode.listening = split.length - 1),
        hydrateNode.setAttribute("data-dioxus-id", id.toString());
      for (let j = 1; j < split.length; j++) {
        let split2 = split[j].split(":"),
          event_name = split2[0],
          bubbles = split2[1] === "1";
        this.createListener(event_name, hydrateNode, bubbles);
      }
    }
  }
  hydrate(ids, underNodes) {
    for (let i = 0; i < underNodes.length; i++) {
      let under = underNodes[i];
      if (under instanceof HTMLElement) {
        if (under.getAttribute("data-node-hydration"))
          this.hydrate_node(under, ids);
        let hydrateNodes = under.querySelectorAll("[data-node-hydration]");
        for (let i2 = 0; i2 < hydrateNodes.length; i2++)
          this.hydrate_node(hydrateNodes[i2], ids);
      }
      let treeWalker = document.createTreeWalker(
          under,
          NodeFilter.SHOW_COMMENT
        ),
        nextSibling = under.nextSibling,
        continueToNextNode = () => {
          if (!treeWalker.nextNode()) return !1;
          return treeWalker.currentNode !== nextSibling;
        };
      while (treeWalker.currentNode) {
        let currentNode = treeWalker.currentNode;
        if (currentNode.nodeType === Node.COMMENT_NODE) {
          let id = currentNode.textContent,
            placeholderSplit = id.split("placeholder");
          if (placeholderSplit.length > 1) {
            if (
              ((this.nodes[ids[parseInt(placeholderSplit[1])]] = currentNode),
              !continueToNextNode())
            )
              break;
            continue;
          }
          let textNodeSplit = id.split("node-id");
          if (textNodeSplit.length > 1) {
            let next = currentNode.nextSibling;
            currentNode.remove();
            let commentAfterText, textNode;
            if (next.nodeType === Node.COMMENT_NODE) {
              let newText = next.parentElement.insertBefore(
                document.createTextNode(""),
                next
              );
              (commentAfterText = next), (textNode = newText);
            } else (textNode = next), (commentAfterText = textNode.nextSibling);
            (treeWalker.currentNode = commentAfterText),
              (this.nodes[ids[parseInt(textNodeSplit[1])]] = textNode);
            let exit = currentNode === under || !continueToNextNode();
            if ((commentAfterText.remove(), exit)) break;
            continue;
          }
        }
        if (!continueToNextNode()) break;
      }
    }
  }
  setAttributeInner(node, field, value, ns) {
    setAttributeInner(node, field, value, ns);
  }
}
export { BaseInterpreter };
