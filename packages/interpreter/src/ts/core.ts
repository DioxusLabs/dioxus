// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

import { setAttributeInner } from "./set_attribute";

export type NodeId = number;

// Element decorated with the listener bookkeeping properties that the
// interpreter attaches at runtime. Saves an `@ts-ignore` per assignment.
interface ListenerElement extends Element {
  listening?: number;
}

// Virtual placeholder: empty `DynamicNode::Placeholder` slots have no DOM
// presence; `nodes[id]` holds one of these sentinels. `before` can chain
// through other sentinels; `materialized` is set when a chained sibling
// later allocates a real text/element so chain walkers can short-circuit.
export interface VirtualPlaceholder {
  __virtual: true;
  parent: Node | null;
  before: Node | VirtualPlaceholder | null;
  materialized?: Node;
}

function isVirtual(x: any): x is VirtualPlaceholder {
  return x !== null && typeof x === "object" && x.__virtual === true;
}

function resolveAnchorBefore(
  before: Node | VirtualPlaceholder | null
): Node | null {
  while (before && isVirtual(before)) {
    // Defensive: `materialized` can point to a node that was removed by a
    // later `Remove` mutation. The anchor itself never sees that removal,
    // so we check `isConnected` and fall through if the cached node has
    // been detached.
    if (before.materialized && before.materialized.isConnected) {
      return before.materialized;
    }
    before = before.before;
  }
  return before as Node | null;
}

type NodeOrVirtual = Node | VirtualPlaceholder;

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

  nodes: NodeOrVirtual[];
  nodeIds: WeakMap<object, number>;
  stack: NodeOrVirtual[];

  // sledgehammer is generating this...
  m: any;

  constructor() {}

  initialize(root: HTMLElement, handler: EventListener | null = null) {
    this.global = {};
    this.local = {};
    this.root = root;

    this.nodes = [root];
    this.nodeIds = new WeakMap();
    this.nodeIds.set(root, 0);
    this.stack = [root];

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
    // Sentinels are returned as-is; Rust `dyn_ref::<Element>()` fails the
    // `instanceof` check and bails. No comment is ever synthesized here.
    return this.nodes[id] as Node;
  }

  pushRoot(node: NodeOrVirtual) {
    this.stack.push(node);
  }

  pushId(id: NodeId) {
    this.stack.push(this.nodes[id]);
  }

  popId(id: NodeId) {
    const node = this.stack.pop();
    if (!node) throw new Error("popId: empty stack");
    this.nodes[id] = node;
    this.nodeIds.set(node as object, id);
  }

  currentTopId(): NodeId {
    const node = this.stack[this.stack.length - 1];
    const id = this.nodeIds.get(node as object);
    if (id === undefined) throw new Error("currentTopId: top node has no ElementId");
    return id;
  }

  child(index: number) {
    const parent = this.stack[this.stack.length - 1] as Node;
    const child = parent.childNodes[index];
    if (!child) throw new Error("child: index out of bounds");
    this.stack[this.stack.length - 1] = child;
  }

  pop() {
    this.stack.pop();
  }

  createElementTop(tag: string, ns: string | null) {
    this.stack.push(ns ? document.createElementNS(ns, tag) : document.createElement(tag));
  }

  createTextTop(text: string) {
    this.stack.push(document.createTextNode(text));
  }

  cloneTop() {
    const node = this.stack[this.stack.length - 1] as Node;
    this.stack[this.stack.length - 1] = node.cloneNode(true);
  }

  clearNodeId(node: NodeOrVirtual) {
    const id = this.nodeIds.get(node as object);
    if (id !== undefined) {
      this.nodes[id] = undefined as any;
      this.nodeIds.delete(node as object);
    }
  }

  appendChildrenToTop(many: number) {
    const parentIdx = this.stack.length - many - 1;
    const parent = this.stack[parentIdx];
    const items = this.stack.splice(parentIdx + 1, many);
    if (isVirtual(parent)) {
      const resolved = this.resolveAnchor(parent);
      this.applyChunk(items, parent.parent as Node, resolved);
      this.materializeAnchor(parent, items);
    } else {
      this.applyChunk(items, parent as Node, null);
    }
  }

  replaceTopWith(many: number) {
    const targetIdx = this.stack.length - many - 1;
    const target = this.stack[targetIdx];
    const items = this.stack.splice(targetIdx + 1, many);
    this.stack.pop();
    this.replaceAtResolvedTarget(target, items);
    this.clearNodeId(target);
  }

  insertAfterTop(many: number) {
    const anchorIdx = this.stack.length - many - 1;
    const anchor = this.stack[anchorIdx];
    const items = this.stack.splice(anchorIdx + 1, many);
    if (isVirtual(anchor)) {
      this.applyChunk(items, anchor.parent as Node, this.resolveAnchor(anchor));
      for (const item of items) {
        if (!isVirtual(item)) {
          anchor.before = item;
          break;
        }
      }
    } else {
      const node = anchor as Node;
      this.applyChunk(items, node.parentNode as Node, node.nextSibling);
    }
  }

  insertBeforeTop(many: number) {
    const anchorIdx = this.stack.length - many - 1;
    const anchor = this.stack[anchorIdx];
    const items = this.stack.splice(anchorIdx + 1, many);
    if (isVirtual(anchor)) {
      this.applyChunk(items, anchor.parent as Node, this.resolveAnchor(anchor));
    } else {
      const node = anchor as Node;
      this.applyChunk(items, node.parentNode as Node, node);
    }
  }

  setTextTop(text: string) {
    const target = this.stack[this.stack.length - 1];
    if (isVirtual(target)) {
      if (text === "") return;
      const node = document.createTextNode(text);
      const insertBefore = this.resolveAnchor(target);
      if (target.parent) {
        if (insertBefore) target.parent.insertBefore(node, insertBefore);
        else target.parent.appendChild(node);
      }
      target.materialized = node;
      const id = this.nodeIds.get(target as object);
      if (id !== undefined) {
        this.nodes[id] = node;
        this.nodeIds.set(node, id);
        this.nodeIds.delete(target as object);
      }
      this.stack[this.stack.length - 1] = node;
    } else {
      (target as Node).textContent = text;
    }
  }

  removeTop() {
    const target = this.stack.pop();
    if (!target) return;
    this.clearNodeId(target);
    if (isVirtual(target)) return;
    const node = target as ListenerElement;
    if (node.listening) this.removeAllNonBubblingListeners(node);
    (node as ChildNode).remove();
  }

  setTopAttribute(field: string, value: string, ns: string | null) {
    this.setAttributeInner(this.stack[this.stack.length - 1] as Node, field, value, ns);
  }

  removeTopAttribute(field: string, ns: string | null) {
    const node = this.stack[this.stack.length - 1] as any;
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
    const node = this.stack[this.stack.length - 1] as ListenerElement;
    const id = this.currentTopId();
    if (node.listening) node.listening += 1;
    else node.listening = 1;
    node.setAttribute("data-dioxus-id", `${id}`);
    this.createListener(event_name, node, bubbles);
  }

  addTopForeignEventListener(event_name: string, bubbles: boolean) {
    const node = this.stack[this.stack.length - 1] as ListenerElement;
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
    const node = this.stack[this.stack.length - 1] as ListenerElement;
    node.listening = (node.listening ?? 1) - 1;
    node.removeAttribute("data-dioxus-id");
    this.removeListener(node, event_name, bubbles);
  }

  createVirtualAnchor(
    parent: Node | null,
    before: Node | VirtualPlaceholder | null
  ): VirtualPlaceholder {
    return {
      __virtual: true,
      parent,
      before,
    };
  }

  resolveAnchor(anchor: VirtualPlaceholder): Node | null {
    return resolveAnchorBefore(anchor.before);
  }

  materializeAnchor(anchor: VirtualPlaceholder, items: NodeOrVirtual[]) {
    for (const it of items) {
      if (!isVirtual(it)) {
        anchor.materialized = it as Node;
        break;
      }
    }
  }

  // Places `items` at `parent`, inserting before `cursorBefore` (or appending
  // when null). Virtual sentinels record their position via `parent`/`before`
  // — the sentinel object is identity-shared with `nodes[id]`, so the mapping
  // updates in place.
  applyChunk(
    items: NodeOrVirtual[],
    parent: Node,
    cursorBefore: Node | null
  ) {
    let cursor = cursorBefore;
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (isVirtual(it)) {
        it.parent = parent;
        it.before = cursor;
      } else {
        cursor = it;
      }
    }
    for (let i = 0; i < items.length; i++) {
      const it = items[i];
      if (isVirtual(it)) continue;
      if (cursorBefore) parent.insertBefore(it, cursorBefore);
      else parent.appendChild(it);
    }
  }

  replaceAtResolvedTarget(
    target: NodeOrVirtual,
    items: NodeOrVirtual[]
  ) {
    if (isVirtual(target)) {
      // Insert at the virtual anchor's logical position, then materialize
      // that anchor with the first real node in the inserted run if there is
      // one. Later chained anchors can then resolve to the right cursor.
      const parent = target.parent as Node;
      const cursor = this.resolveAnchor(target);
      this.applyChunk(items, parent, cursor);
      this.materializeAnchor(target, items);
    } else {
      const real = target as ListenerElement;
      if (real.listening) this.removeAllNonBubblingListeners(real);
      const parent = real.parentNode as Node;
      const next = real.nextSibling;
      (real as ChildNode).remove();
      this.applyChunk(items, parent, next);
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
