// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

import { setAttributeInner } from "./set_attribute";

export type NodeId = number;

// Element decorated with the listener bookkeeping properties that the
// interpreter attaches at runtime.
interface ListenerElement extends Element {
  listening?: number;
}

// A stack entry pairs a DOM node with the ElementId it was pushed under, or
// `null` for nodes pushed positionally (e.g. cloned template children).
type StackEntry = [Node, NodeId | null];

interface ListenerRecord {
  bubbles: boolean;
  count: number;
  callback: EventListener | null;
}

export class BaseInterpreter {
  // non bubbling events listen at the element the listener was created at
  global: {
    [key: string]: { active: number; callback: EventListener };
  };
  // Weak metadata follows physical nodes, independently of reused ElementIds.
  private nodeIds: WeakMap<Node, Set<NodeId>>;
  private listeners: WeakMap<Element, Map<string, ListenerRecord>>;
  private queuedMounted: [NodeId, Node][];

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
    this.nodeIds = new WeakMap();
    this.listeners = new WeakMap();
    this.queuedMounted = [];
    this.root = root;

    this.nodes = [];
    this.setNode(0, root);
    this.stack = [[root, 0]];

    this.handler = handler;

    // make sure to set the root element's ID so it still registers events
    root.setAttribute("data-dioxus-id", "0");
  }

  handleResizeEvent(entry: ResizeObserverEntry) {
    const target = entry.target;
    if (!this.listeners.get(target)?.has("resize")) return;

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
    if (!this.listeners.get(target)?.has("visible")) return;

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

  createListener(event_name: string, element: ListenerElement, bubbles: boolean) {
    let records = this.listeners.get(element);
    if (!records) {
      records = new Map();
      this.listeners.set(element, records);
    }
    let record = records.get(event_name);
    if (record) {
      record.count++;
    } else {
      // Native mounted registrations route IPC immediately; there is no DOM event.
      record = { bubbles, count: 1, callback: event_name === "mounted" ? null : this.handler };
      records.set(event_name, record);
      if (!bubbles && record.callback) element.addEventListener(event_name, record.callback);
      if (event_name === "resize") this.createResizeObserver(element);
      else if (event_name === "visible") this.createIntersectionObserver(element);
    }
    element.listening = (element.listening || 0) + 1;

    if (bubbles) {
      if (this.global[event_name] === undefined) {
        this.global[event_name] = { active: 1, callback: this.handler };
        this.root.addEventListener(event_name, this.handler);
      } else {
        this.global[event_name].active++;
      }
    }
  }

  removeListener(element: ListenerElement, event_name: string) {
    const records = this.listeners.get(element);
    const record = records?.get(event_name);
    if (!record) return;

    if (record.bubbles) this.removeBubblingListener(event_name);
    if (--record.count === 0) {
      if (!record.bubbles && record.callback) element.removeEventListener(event_name, record.callback);
      if (event_name === "resize") this.removeResizeObserver(element);
      else if (event_name === "visible") this.removeIntersectionObserver(element);
      records.delete(event_name);
      if (records.size === 0) this.listeners.delete(element);
    }
    element.listening--;
    if (element.listening === 0) element.removeAttribute("data-dioxus-id");
  }

  removeBubblingListener(event_name: string) {
    const listener = this.global[event_name];
    if (--listener.active === 0) {
      this.root.removeEventListener(event_name, listener.callback);
      delete this.global[event_name];
    }
  }

  // All assignments, including hydration, must register the reverse association.
  setNode(id: NodeId, node: Node) {
    const previous = this.nodes[id];
    if (previous) {
      const previousIds = this.nodeIds.get(previous);
      previousIds?.delete(id);
      if (previousIds?.size === 0) this.nodeIds.delete(previous);
    }
    this.nodes[id] = node;
    let ids = this.nodeIds.get(node);
    if (!ids) {
      ids = new Set();
      this.nodeIds.set(node, ids);
    }
    ids.add(id);
  }

  // Only call for a subtree retired by Remove/Replace, after moving replacement
  // operands out of it. Disconnected templates and pending stack nodes are live.
  retireSubtree(root: Node) {
    const pending = [root];
    while (pending.length) {
      const node = pending.pop();
      // The renderer root (ElementId 0) is not an ordinary retired subtree.
      if (node === this.root) continue;
      for (const child of node.childNodes) pending.push(child);

      if (node.nodeType === Node.ELEMENT_NODE) {
        const element = node as ListenerElement;
        const records = this.listeners.get(element);
        if (records) {
          for (const [name, record] of records) {
            if (record.bubbles) {
              for (let i = 0; i < record.count; i++) this.removeBubblingListener(name);
            } else if (record.callback) {
              element.removeEventListener(name, record.callback);
            }
            if (name === "resize") this.removeResizeObserver(element);
            else if (name === "visible") this.removeIntersectionObserver(element);
          }
          this.listeners.delete(element);
        }
        element.removeAttribute("data-dioxus-id");
        delete element.listening;
      }
      const ids = this.nodeIds.get(node);
      if (ids) {
        for (const id of ids) {
          // The slot may have been rebound before the old physical node retired.
          if (id !== 0 && this.nodes[id] === node) this.nodes[id] = undefined;
        }
        this.nodeIds.delete(node);
      }
    }
  }

  queueMounted(id: NodeId) {
    this.queuedMounted.push([id, this.nodes[id]]);
  }

  queueTopMounted() {
    const [node] = this.stack[this.stack.length - 1];
    this.queuedMounted.push([this.currentTopId(), node]);
  }

  takeMountedIds(): Uint32Array {
    const queued = this.queuedMounted;
    this.queuedMounted = [];
    return Uint32Array.from(
      queued.filter(([id, node]) => node && this.nodes[id] === node),
      ([id]) => id
    );
  }

  getNode(id: NodeId): Node {
    return this.nodes[id];
  }

  // Attach an event listener to a previously-bound node. Mirrors
  // `addTopEventListener`, but addresses the element by id rather than the
  // working stack, so the Rust hydration cursor can drive it directly.
  setNodeListener(id: NodeId, event_name: string, bubbles: boolean) {
    const node = this.nodes[id] as ListenerElement;
    node.setAttribute("data-dioxus-id", `${id}`);
    this.createListener(event_name, node, bubbles);
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
    this.setNode(id, entry[0]);
  }

  setId(id: NodeId) {
    const top = this.stack[this.stack.length - 1];
    if (!top) throw new Error("setId: empty stack");
    this.setNode(id, top[0]);
    top[1] = id;
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
    this.stack.push([
      ns ? document.createElementNS(ns, tag) : document.createElement(tag),
      null,
    ]);
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
    const parent = target.parentNode as Node;
    const next = target.nextSibling;
    (target as ChildNode).remove();
    this.applyChunk(items, parent, next);
    if (!items.some(([node]) => node === target)) this.retireSubtree(target);
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
    (node as ChildNode).remove();
    this.retireSubtree(node);
  }

  setTopAttribute(field: string, value: string, ns: string | null) {
    this.setAttributeInner(
      this.stack[this.stack.length - 1][0],
      field,
      value,
      ns,
    );
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
    node.setAttribute("data-dioxus-id", `${id}`);
    this.createListener(event_name, node, bubbles);
  }

  addTopForeignEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1][0] as ListenerElement;
    const id = this.currentTopId();
    node.setAttribute("data-dioxus-id", `${id}`);

    this.createListener(event_name, node, bubbles);
    if (event_name === "mounted") {
      (window as any).ipc.postMessage(
        this.sendSerializedEvent({
          name: event_name,
          element: id,
          data: null,
          bubbles,
        }),
      );
    }
  }

  removeTopEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1][0] as ListenerElement;
    this.removeListener(node, event_name);
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
    ns: string | null,
  ) {
    setAttributeInner(node as HTMLElement, field, value, ns ?? "");
  }
}
