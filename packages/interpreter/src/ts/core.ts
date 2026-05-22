// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

import { setAttributeInner } from "./set_attribute";

export type NodeId = number;

// Element decorated with the listener bookkeeping properties that the
// interpreter attaches at runtime. Saves an `@ts-ignore` per assignment.
interface ListenerElement extends Element {
  listening?: number;
}

// Stack-only sentinel for binary-protocol template construction; consumed
// when `append_children_to_top` records the slot position in the parent.
interface TemplateSlotSentinel {
  __dxTemplateSlot: true;
}
function isTemplateSlotSentinel(x: any): x is TemplateSlotSentinel {
  return x !== null && typeof x === "object" && x.__dxTemplateSlot === true;
}

interface TemplateBuildElement extends Element {
  __dxLocalSlots?: number[];
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

// A saved template root: clone with all `TemplateNode::Dynamic` positions
// omitted, plus the slot metadata that lets byte-path walking still locate
// dynamic positions. `slotMap` keys are comma-joined parent paths.
interface TemplateRoot extends Node {
  __dxSlotMap?: Map<string, Set<number>>;
  __dxSlotPaths?: number[][];
}
interface TemplateClone extends Node {
  __dxSlotMap?: Map<string, Set<number>>;
  __dxSlotPaths?: number[][];
  __dxSlotAnchors?: Map<string, VirtualPlaceholder>;
}

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
  stack: NodeOrVirtual[];
  templates: {
    [key: number]: Node[];
  };

  // sledgehammer is generating this...
  m: any;

  constructor() {}

  initialize(root: HTMLElement, handler: EventListener | null = null) {
    this.global = {};
    this.local = {};
    this.root = root;

    this.nodes = [root];
    this.stack = [root];
    this.templates = {};

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

  appendChildren(id: NodeId, many: number) {
    const root = this.nodes[id] as Node;
    const els = this.stack.splice(this.stack.length - many);
    this.applyChunk(els, root, null);
  }

  // Places `items` at `parent`, inserting before `cursorBefore` (or appending
  // when null). Virtual sentinels record their position via `parent`/`before`
  // — the sentinel object is identity-shared with `nodes[id]`, so the
  // mapping updates in place. Binary-protocol template-slot sentinels (from
  // `add_placeholder`) are skipped here — their bookkeeping happened in
  // `recordTemplateSlots` before this call.
  applyChunk(
    items: (NodeOrVirtual | TemplateSlotSentinel)[],
    parent: Node,
    cursorBefore: Node | null
  ) {
    let cursor = cursorBefore;
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (isTemplateSlotSentinel(it)) continue;
      if (isVirtual(it)) {
        it.parent = parent;
        it.before = cursor;
      } else {
        cursor = it as Node;
      }
    }
    for (let i = 0; i < items.length; i++) {
      const it = items[i];
      if (isTemplateSlotSentinel(it) || isVirtual(it)) continue;
      if (cursorBefore) parent.insertBefore(it as Node, cursorBefore);
      else parent.appendChild(it as Node);
    }
  }

  // Binary-protocol path: before `append_children_to_top` calls
  // `applyChunk`, record any slot-sentinel positions in template-tree order
  // on the parent so `finalizeTemplateRoots` can later compute the per-root
  // slot map. Non-sentinel items advance the template index by 1; sentinels
  // claim their template-index and stay in `__dxLocalSlots`.
  recordTemplateSlots(
    parent: TemplateBuildElement,
    items: (NodeOrVirtual | TemplateSlotSentinel)[]
  ) {
    let hasSlot = false;
    for (const it of items) if (isTemplateSlotSentinel(it)) { hasSlot = true; break; }
    if (!hasSlot) return;
    if (!parent.__dxLocalSlots) parent.__dxLocalSlots = [];
    let templateIdx = 0;
    for (const it of items) {
      if (isTemplateSlotSentinel(it)) {
        parent.__dxLocalSlots.push(templateIdx);
      }
      templateIdx++;
    }
  }

  // Compile the binary-protocol-built template roots. Walks each root,
  // gathering `__dxLocalSlots` into a per-root `__dxSlotMap` and
  // `__dxSlotPaths`, then clears the per-element `__dxLocalSlots` annotation
  // so it can't leak into clones. Roots that are themselves slot sentinels
  // (a `TemplateNode::Dynamic` at top level) become `null` entries — they
  // are never cloned.
  finalizeTemplateRoots(
    roots: (NodeOrVirtual | TemplateSlotSentinel)[]
  ): (Node | null)[] {
    const result: (Node | null)[] = [];
    for (const root of roots) {
      if (isTemplateSlotSentinel(root)) {
        result.push(null);
        continue;
      }
      if (isVirtual(root)) {
        result.push(null);
        continue;
      }
      const r = root as TemplateRoot & TemplateBuildElement;
      const slotPaths: number[][] = [];
      const slotMap = new Map<string, Set<number>>();
      this.gatherTemplateSlots(r, [], slotPaths, slotMap);
      if (slotPaths.length > 0) {
        r.__dxSlotPaths = slotPaths;
        r.__dxSlotMap = slotMap;
      }
      result.push(r);
    }
    return result;
  }

  // Depth-first walk an in-construction template root. At each element we
  // read its `__dxLocalSlots` (template-tree-order indices of slot children),
  // emit the corresponding slot path, and recurse into real children by
  // recomputing template-tree indices that account for the slot gaps.
  gatherTemplateSlots(
    node: Node,
    pathPrefix: number[],
    slotPaths: number[][],
    slotMap: Map<string, Set<number>>
  ) {
    const el = node as TemplateBuildElement;
    const localSlots = el.__dxLocalSlots;
    if (localSlots && localSlots.length > 0) {
      const key = pathPrefix.join(",");
      let set = slotMap.get(key);
      if (!set) {
        set = new Set();
        slotMap.set(key, set);
      }
      for (const idx of localSlots) {
        set.add(idx);
        slotPaths.push([...pathPrefix, idx]);
      }
      // Clear the annotation so cloned templates don't carry stale state.
      el.__dxLocalSlots = undefined;
    }
    // Recurse through real children. Their template-tree indices are
    // computed by skipping over slot positions.
    if (!node.firstChild) return;
    const slotSet = localSlots ? new Set(localSlots) : null;
    let realChild: Node | null = node.firstChild;
    let templateIdx = 0;
    while (realChild) {
      while (slotSet && slotSet.has(templateIdx)) templateIdx++;
      this.gatherTemplateSlots(
        realChild,
        [...pathPrefix, templateIdx],
        slotPaths,
        slotMap
      );
      realChild = realChild.nextSibling;
      templateIdx++;
    }
  }

  replaceWithChunk(id: NodeId, m: number) {
    const root = this.nodes[id];
    const items = this.stack.splice(this.stack.length - m);
    if (isVirtual(root)) {
      this.applyChunk(items, root.parent as Node, this.resolveAnchor(root));
      (this.nodes as any)[id] = undefined;
    } else {
      const node = root as ListenerElement;
      if (node.listening) this.removeAllNonBubblingListeners(node);
      // Remove the anchor before placing the chunk so virtual sentinels in
      // `items` record a live cursor instead of one to a detached node.
      const parent = (node as Node).parentNode as Node;
      const next = (node as Node).nextSibling;
      (node as ChildNode).remove();
      this.applyChunk(items, parent, next);
    }
  }

  insertChunkAfter(id: NodeId, m: number) {
    const root = this.nodes[id];
    const items = this.stack.splice(this.stack.length - m);
    if (isVirtual(root)) {
      this.applyChunk(items, root.parent as Node, this.resolveAnchor(root));
      // The placeholder now sits before the new run; its `before` follows the
      // first real new item so a later "after" insert lands on the right side.
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        if (!isVirtual(it)) {
          root.before = it as Node;
          break;
        }
      }
    } else {
      const node = root as Node;
      this.applyChunk(items, node.parentNode as Node, node.nextSibling);
    }
  }

  insertChunkBefore(id: NodeId, m: number) {
    const root = this.nodes[id];
    const items = this.stack.splice(this.stack.length - m);
    if (isVirtual(root)) {
      // Placeholder sits after the new run; `before` is unchanged.
      this.applyChunk(items, root.parent as Node, this.resolveAnchor(root));
    } else {
      const node = root as Node;
      this.applyChunk(items, node.parentNode as Node, node);
    }
  }

  // Lazy materialization for virtualized dynamic text sentinels emitted by
  // the walker's `SynthText`/`SynthTextAfter` ops. Empty updates stay
  // virtual; the first non-empty `setNodeText` allocates a real text node at
  // the sentinel's chain position, then later updates flow through the
  // standard `.textContent =` path.
  setNodeText(id: NodeId, text: string) {
    const node = this.nodes[id];
    if (isVirtual(node)) {
      if (text === "") return;
      const t = document.createTextNode(text);
      const insertBefore = this.resolveAnchor(node);
      if (node.parent) {
        if (insertBefore) node.parent.insertBefore(t, insertBefore);
        else node.parent.appendChild(t);
      }
      node.materialized = t;
      this.nodes[id] = t;
      return;
    }
    (node as Text).textContent = text;
  }

  removeNode(id: NodeId) {
    const node = this.nodes[id];
    if (node === undefined) return;
    if (isVirtual(node)) {
      (this.nodes as any)[id] = undefined;
      return;
    }
    const el = node as ListenerElement;
    if (el.listening) this.removeAllNonBubblingListeners(el);
    (node as ChildNode).remove();
  }

  // Resolves `path` to either a real DOM node (attribute target, etc.) or to
  // a pre-built virtual placeholder sentinel (a Dynamic slot). When the
  // result is a slot anchor we `applyChunk` at its `(parent, before)` instead
  // of trying to remove a DOM node — the marker no longer exists.
  replacePlaceholderPath(ptr: number, len: number, m: number) {
    const items = this.stack.splice(this.stack.length - m);
    const node = this.loadChild(ptr, len);
    this.replaceAtResolvedTarget(node, items);
  }

  // Internal shared between `replacePlaceholderPath` (ptr+len) and the
  // binary-protocol byte-array variant.
  replaceAtResolvedTarget(
    target: NodeOrVirtual,
    items: NodeOrVirtual[]
  ) {
    if (isVirtual(target)) {
      // Pre-built slot anchor from `prepareTemplateClone`. Insert items at
      // its logical position, then "materialize" the anchor: stash a
      // reference to the first real node of the run (if any) on
      // `anchor.materialized` so chain-walking siblings (an earlier slot
      // whose `before` points at this anchor) can find the right
      // insertion cursor at their own replace time. If the run is empty
      // of real nodes, leave `materialized` unset; `resolveAnchorBefore` will
      // continue walking through `anchor.before` to find the next live
      // position.
      const parent = target.parent as Node;
      const cursor = this.resolveAnchor(target);
      this.applyChunk(items, parent, cursor);
      this.materializeAnchor(target, items);
    } else {
      const real = target as Node;
      const parent = real.parentNode as Node;
      const next = real.nextSibling;
      (real as ChildNode).remove();
      this.applyChunk(items, parent, next);
    }
  }

  // Slot-aware path walk: each step indexes into template-tree children,
  // skipping Dynamic slot positions (which have no DOM presence). When
  // `treatLeafAsSlot` is true a slot at the final step resolves to its
  // pre-built virtual sentinel; otherwise it's treated as a slot-in-middle
  // (structurally invalid for byte paths since Dynamic has no children).
  walkSlotPath(
    root: TemplateClone,
    step: (i: number) => number,
    len: number,
    treatLeafAsSlot: boolean
  ): NodeOrVirtual | null {
    const slotMap = root.__dxSlotMap;
    const slotAnchors = root.__dxSlotAnchors;
    let node: Node = root;
    const prefixParts: number[] = [];
    for (let i = 0; i < len; i++) {
      const s = step(i);
      const slotsHere = slotMap?.get(prefixParts.join(","));
      if (slotsHere && slotsHere.has(s)) {
        if (treatLeafAsSlot && i === len - 1) {
          prefixParts.push(s);
          const anchor = slotAnchors?.get(prefixParts.join(","));
          if (!anchor) throw new Error("loadChild: missing slot anchor");
          return anchor;
        }
        if (treatLeafAsSlot) {
          throw new Error("loadChild: byte path traverses a slot position");
        }
      }
      let slotsBefore = 0;
      if (slotsHere) {
        for (const slot of slotsHere) if (slot < s) slotsBefore++;
      }
      let realStep = s - slotsBefore;
      let child: Node | null = node.firstChild;
      while (realStep > 0 && child) {
        child = child.nextSibling;
        realStep--;
      }
      if (!child) return null;
      node = child;
      prefixParts.push(s);
    }
    return node;
  }

  loadChild(ptr: number, len: number): NodeOrVirtual {
    const root = this.stack[this.stack.length - 1] as TemplateClone;
    return this.walkSlotPath(
      root,
      (i) => this.m.getUint8(ptr + i),
      len,
      true
    ) as NodeOrVirtual;
  }

  loadChildBytes(array: Uint8Array | number[]): NodeOrVirtual {
    const root = this.stack[this.stack.length - 1] as TemplateClone;
    return this.walkSlotPath(
      root,
      (i) => array[i],
      array.length,
      true
    ) as NodeOrVirtual;
  }

  replacePlaceholderPathBytes(array: Uint8Array | number[], n: number) {
    const items = this.stack.splice(this.stack.length - n);
    const target = this.loadChildBytes(array);
    this.replaceAtResolvedTarget(target, items);
  }

  // Clone the saved template root and reattach its slot metadata so
  // `loadChild` can reinterpret subsequent byte paths. `__dxSlotAnchors` is
  // built fresh per clone (the sentinels reference live nodes).
  prepareTemplateClone(tmpl_id: number, index: number): Node {
    const saved = this.templates[tmpl_id][index] as TemplateRoot;
    const clone = saved.cloneNode(true) as TemplateClone;
    const slotMap = saved.__dxSlotMap;
    const slotPaths = saved.__dxSlotPaths;
    if (slotMap) clone.__dxSlotMap = slotMap;
    if (slotPaths) clone.__dxSlotPaths = slotPaths;
    if (slotPaths && slotPaths.length > 0) {
      clone.__dxSlotAnchors = this.buildSlotAnchors(
        clone,
        slotPaths,
        slotMap as Map<string, Set<number>>
      );
    }
    return clone;
  }

  buildSlotAnchors(
    cloneRoot: Node,
    slotPaths: number[][],
    slotMap: Map<string, Set<number>>
  ): Map<string, VirtualPlaceholder> {
    const anchors = new Map<string, VirtualPlaceholder>();
    // Group slots by parent path so we can chain right-to-left.
    const byParent = new Map<string, { key: string; path: number[]; leaf: number }[]>();
    for (const path of slotPaths) {
      if (path.length === 0) continue;
      const parentKey = path.slice(0, -1).join(",");
      const key = path.join(",");
      const leaf = path[path.length - 1];
      let list = byParent.get(parentKey);
      if (!list) {
        list = [];
        byParent.set(parentKey, list);
      }
      list.push({ key, path, leaf });
    }
    for (const [parentKey, slotList] of byParent.entries()) {
      const parentPath = parentKey === "" ? [] : parentKey.split(",").map(Number);
      const parentNode = this.walkSlotPath(
        cloneRoot as TemplateClone,
        (i) => parentPath[i],
        parentPath.length,
        false
      ) as Node | null;
      if (!parentNode) continue;
      const parentSlots = slotMap.get(parentKey);
      if (!parentSlots) continue;
      slotList.sort((a, b) => a.leaf - b.leaf);
      // Chain adjacent slots right-to-left so each new anchor can reference
      // its later sibling. Static siblings break the chain because the
      // earlier slot must insert before that real node, not wherever the
      // later slot eventually materializes.
      for (let i = slotList.length - 1; i >= 0; i--) {
        const { key, leaf } = slotList[i];
        let slotsBefore = 0;
        for (const s of parentSlots) if (s < leaf) slotsBefore++;
        const realBeforeIdx = leaf - slotsBefore;
        const realChildren = parentNode.childNodes;
        const nextReal: Node | null =
          realBeforeIdx < realChildren.length ? realChildren[realBeforeIdx] : null;
        const nextSlot = slotList[i + 1];
        const nextAnchor =
          nextSlot?.leaf === leaf + 1 ? anchors.get(nextSlot.key) ?? null : null;
        const anchor = this.createVirtualAnchor(parentNode, nextAnchor ?? nextReal);
        anchors.set(key, anchor);
      }
    }
    return anchors;
  }

  // `nodes[i]` is null for root-level Dynamic templates (never cloned); the
  // sparse index keeps `load_template(_, index, _)` lookups aligned. Each
  // path in `slotPaths` is `[root_idx, child0, …]` from
  // `Template::node_paths()`; we bucket per-root and attach `__dxSlotMap` +
  // `__dxSlotPaths` for `loadChild` and `prepareTemplateClone` to consume.
  saveTemplate(
    nodes: (TemplateRoot | null)[],
    tmpl_id: number,
    slotPaths?: Uint8Array[]
  ) {
    this.templates[tmpl_id] = nodes as Node[];
    if (!slotPaths || slotPaths.length === 0) return;

    // Bucket each slot path by its root index.
    const perRoot: number[][][] = nodes.map(() => []);
    for (let i = 0; i < slotPaths.length; i++) {
      const p = slotPaths[i];
      if (p.length === 0) continue;
      const rootIdx = p[0];
      if (rootIdx >= perRoot.length) continue;
      const rel = new Array<number>(p.length - 1);
      for (let k = 1; k < p.length; k++) rel[k - 1] = p[k];
      perRoot[rootIdx].push(rel);
    }
    for (let r = 0; r < nodes.length; r++) {
      const root = nodes[r];
      const paths = perRoot[r];
      if (!root || paths.length === 0) continue;
      root.__dxSlotPaths = paths;
      const map = new Map<string, Set<number>>();
      for (const path of paths) {
        if (path.length === 0) continue;
        const key = path.slice(0, -1).join(",");
        let set = map.get(key);
        if (!set) {
          set = new Set();
          map.set(key, set);
        }
        set.add(path[path.length - 1]);
      }
      root.__dxSlotMap = map;
    }
  }

  setAttributeInner(
    node: HTMLElement,
    field: string,
    value: string,
    ns: string
  ) {
    setAttributeInner(node, field, value, ns);
  }
}
