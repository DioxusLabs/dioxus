// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

import { setAttributeInner } from "./set_attribute";

export type NodeId = number;

// Element decorated with the listener bookkeeping properties that the
// interpreter attaches at runtime. Saves an `@ts-ignore` per assignment.
interface ListenerElement extends Element {
  listening?: number;
}

// A stack entry pairs a DOM node with the ElementId it was pushed under, or
// `null` for nodes pushed positionally (e.g. cloned template children).
type StackEntry = [Node, NodeId | null];

export class BaseInterpreter {
  // non bubbling events listen at the element the listener was created at
  global: {
    [key: string]: { active: number; callback: EventListener };
  };
  // bubbling events can listen at the root element
  local: {
    [key: string]: {
      [key: string]: EventListener;
    };
  };

  root: HTMLElement;
  handler: EventListener;
  resizeObserver: ResizeObserver;
  intersectionObserver: IntersectionObserver;

  nodes: Node[];
  stack: StackEntry[];

  // sledgehammer is generating this...
  m: any;

  constructor() {}

  initialize(root: HTMLElement, handler: EventListener | null = null) {
    this.global = {};
    this.local = {};
    this.root = root;

    this.nodes = [root];
    this.stack = [[root, 0]];

    this.handler = handler;

    // make sure to set the root element's ID so it still registers events
    root.setAttribute("data-dioxus-id", "0");
  }

  handleResizeEvent(entry: ResizeObserverEntry) {
    const target = entry.target;

    let event = new CustomEvent<ResizeObserverEntry>("resize", {
      bubbles: false,
      detail: entry,
    });

    target.dispatchEvent(event);
  }

  createResizeObserver(element: Element) {
    // Lazily create the resize observer
    if (!this.resizeObserver) {
      this.resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          this.handleResizeEvent(entry);
        }
      });
    }
    this.resizeObserver.observe(element);
  }

  removeResizeObserver(element: Element) {
    if (this.resizeObserver) {
      this.resizeObserver.unobserve(element);
    }
  }

  handleIntersectionEvent(entry: IntersectionObserverEntry) {
    const target = entry.target;

    let event = new CustomEvent<IntersectionObserverEntry>("visible", {
      bubbles: false,
      detail: entry,
    });

    target.dispatchEvent(event);
  }

  createIntersectionObserver(element: Element) {
    /// Lazily create the intersection observer
    if (!this.intersectionObserver) {
      this.intersectionObserver = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          this.handleIntersectionEvent(entry);
        }
      });
    }
    this.intersectionObserver.observe(element);
  }

  removeIntersectionObserver(element: Element) {
    if (this.intersectionObserver) {
      this.intersectionObserver.unobserve(element);
    }
  }

  createListener(event_name: string, element: Element, bubbles: boolean) {
    if (event_name == "resize") {
      this.createResizeObserver(element);
    } else if (event_name == "visible") {
      this.createIntersectionObserver(element);
    }

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

  removeListener(element: Element, event_name: string, bubbles: boolean) {
    if (event_name == "resize") {
      this.removeResizeObserver(element);
    } else if (event_name == "visible") {
      this.removeIntersectionObserver(element);
    } else if (bubbles) {
      this.removeBubblingListener(event_name);
    } else {
      this.removeNonBubblingListener(element, event_name);
    }
  }

  removeBubblingListener(event_name: string) {
    this.global[event_name].active--;
    if (this.global[event_name].active === 0) {
      this.root.removeEventListener(
        event_name,
        this.global[event_name].callback
      );
      delete this.global[event_name];
    }
  }

  removeNonBubblingListener(element: Element, event_name: string) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id][event_name];
    if (Object.keys(this.local[id]).length === 0) {
      delete this.local[id];
    }
    element.removeEventListener(event_name, this.handler);
  }

  removeAllNonBubblingListeners(element: Element) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }

  getNode(id: NodeId): Node {
    return this.nodes[id];
  }

  setNode(id: NodeId, node: Node) {
    this.nodes[id] = node;
  }

  pushRoot(node: Node) {
    this.stack.push([node, null]);
  }

  pushId(id: NodeId) {
    this.stack.push([this.nodes[id], id]);
  }

  popId(id: NodeId) {
    const entry = this.stack.pop();
    if (!entry) throw new Error("popId: empty stack");
    this.nodes[id] = entry[0];
  }

  currentTopId(): NodeId {
    const id = this.stack[this.stack.length - 1][1];
    if (id == null) throw new Error("currentTopId: top node has no ElementId");
    return id;
  }

  child(index: number) {
    const parent = this.stack[this.stack.length - 1][0];
    const child = parent.childNodes[index];
    if (!child) throw new Error("child: index out of bounds");
    this.stack[this.stack.length - 1] = [child, null];
  }

  pop() {
    this.stack.pop();
  }

  createElementTop(tag: string, ns: string | null) {
    this.stack.push([ns ? document.createElementNS(ns, tag) : document.createElement(tag), null]);
  }

  createTextTop(text: string) {
    this.stack.push([document.createTextNode(text), null]);
  }

  cloneTop() {
    const node = this.stack[this.stack.length - 1][0];
    this.stack[this.stack.length - 1] = [node.cloneNode(true), null];
  }

  appendChildrenToTop(many: number) {
    const parentIdx = this.stack.length - many - 1;
    const parent = this.stack[parentIdx][0];
    const items = this.stack.splice(parentIdx + 1, many);
    this.applyChunk(items, parent, null);
  }

  replaceTopWith(many: number) {
    const targetIdx = this.stack.length - many - 1;
    const target = this.stack[targetIdx][0];
    const items = this.stack.splice(targetIdx + 1, many);
    this.stack.pop();
    const real = target as ListenerElement;
    if (real.listening) this.removeAllNonBubblingListeners(real);
    const parent = target.parentNode as Node;
    const next = target.nextSibling;
    (target as ChildNode).remove();
    this.applyChunk(items, parent, next);
  }

  insertAfterTop(many: number) {
    const anchorIdx = this.stack.length - many - 1;
    const anchor = this.stack[anchorIdx][0];
    const items = this.stack.splice(anchorIdx + 1, many);
    this.applyChunk(items, anchor.parentNode as Node, anchor.nextSibling);
  }

  insertBeforeTop(many: number) {
    const anchorIdx = this.stack.length - many - 1;
    const anchor = this.stack[anchorIdx][0];
    const items = this.stack.splice(anchorIdx + 1, many);
    this.applyChunk(items, anchor.parentNode as Node, anchor);
  }

  setTextTop(text: string) {
    this.stack[this.stack.length - 1][0].textContent = text;
  }

  removeTop() {
    const targetEntry = this.stack.pop();
    if (!targetEntry) return;
    const node = targetEntry[0] as ListenerElement;
    if (node.listening) this.removeAllNonBubblingListeners(node);
    (node as ChildNode).remove();
  }

  setTopAttribute(field: string, value: string, ns: string | null) {
    this.setAttributeInner(this.stack[this.stack.length - 1][0], field, value, ns);
  }

  removeTopAttribute(field: string, ns: string | null) {
    const node = this.stack[this.stack.length - 1][0] as any;
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
  }

  addTopEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1][0] as ListenerElement;
    const id = this.currentTopId();
    if (node.listening) node.listening += 1;
    else node.listening = 1;
    node.setAttribute("data-dioxus-id", `${id}`);
    this.createListener(event_name, node, bubbles);
  }

  addTopForeignEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1][0] as ListenerElement;
    const id = this.currentTopId();
    if (node.listening) node.listening += 1;
    else node.listening = 1;
    node.setAttribute("data-dioxus-id", `${id}`);

    if (event_name === "mounted") {
      (window as any).ipc.postMessage(
        this.sendSerializedEvent({
          name: event_name,
          element: id,
          data: null,
          bubbles,
        })
      );
    } else {
      this.createListener(event_name, node, bubbles);
    }
  }

  removeTopEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1][0] as ListenerElement;
    node.listening = (node.listening ?? 1) - 1;
    node.removeAttribute("data-dioxus-id");
    this.removeListener(node, event_name, bubbles);
  }

  // Insert each node in `items` into `parent` before `cursorBefore`, appending
  // when `cursorBefore` is null. Insertion order is preserved.
  applyChunk(items: StackEntry[], parent: Node, cursorBefore: Node | null) {
    for (const [node] of items) {
      parent.insertBefore(node, cursorBefore);
    }
  }

  setAttributeInner(
    node: Node,
    field: string,
    value: string,
    ns: string | null
  ) {
    setAttributeInner(node as HTMLElement, field, value, ns ?? "");
  }
}
