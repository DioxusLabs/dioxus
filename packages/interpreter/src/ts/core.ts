// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

import { setAttributeInner } from "./set_attribute";

export type NodeId = number;

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
  nodes: Node[];
  stack: Node[];
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

    if (handler) {
      this.handler = handler;
    }
  }

  createListener(event_name: string, element: HTMLElement, bubbles: boolean) {
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

  removeListener(element: HTMLElement, event_name: string, bubbles: boolean) {
    if (bubbles) {
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

  removeNonBubblingListener(element: HTMLElement, event_name: string) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id][event_name];
    if (Object.keys(this.local[id]).length === 0) {
      delete this.local[id];
    }
    element.removeEventListener(event_name, this.handler);
  }

  removeAllNonBubblingListeners(element: HTMLElement) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }

  getNode(id: NodeId): Node {
    return this.nodes[id];
  }

  pushRoot(node: Node) {
    this.stack.push(node);
  }

  appendChildren(id: NodeId, many: number) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0; k < many; k++) {
      root.appendChild(els[k]);
    }
  }

  loadChild(ptr: number, len: number): Node {
    // iterate through each number and get that child
    let node = this.stack[this.stack.length - 1] as Node;
    let ptr_end = ptr + len;

    for (; ptr < ptr_end; ptr++) {
      let end = this.m.getUint8(ptr);
      for (node = node.firstChild; end > 0; end--) {
        node = node.nextSibling;
      }
    }

    return node;
  }

  saveTemplate(nodes: HTMLElement[], tmpl_id: number) {
    this.templates[tmpl_id] = nodes;
  }

  hydrate_node(hydrateNode: HTMLElement, ids: { [key: number]: number }) {
    const hydration = hydrateNode.getAttribute("data-node-hydration");
    const split = hydration!.split(",");
    const id = ids[parseInt(split[0])];

    this.nodes[id] = hydrateNode;

    if (split.length > 1) {
      // @ts-ignore
      hydrateNode.listening = split.length - 1;
      hydrateNode.setAttribute("data-dioxus-id", id.toString());
      for (let j = 1; j < split.length; j++) {
        const listener = split[j];
        const split2 = listener.split(":");
        const event_name = split2[0];
        const bubbles = split2[1] === "1";
        this.createListener(event_name, hydrateNode, bubbles);
      }
    }
  }

  hydrate(ids: { [key: number]: number }, underNodes: Node[]) {
    for (let i = 0; i < underNodes.length; i++) {
      const under = underNodes[i];
      if (under instanceof HTMLElement) {
        if (under.getAttribute("data-node-hydration")) {
          this.hydrate_node(under, ids);
        }
        const hydrateNodes = under.querySelectorAll("[data-node-hydration]");

        for (let i = 0; i < hydrateNodes.length; i++) {
          this.hydrate_node(hydrateNodes[i] as HTMLElement, ids);
        }
      }

      const treeWalker = document.createTreeWalker(
        under,
        NodeFilter.SHOW_COMMENT
      );

      while (treeWalker.currentNode) {
        const currentNode = treeWalker.currentNode as ChildNode;
        if (currentNode.nodeType === Node.COMMENT_NODE) {
          const id = currentNode.textContent!;

          // First try to hydrate the comment node as a placeholder
          const placeholderSplit = id.split("placeholder");

          if (placeholderSplit.length > 1) {
            this.nodes[ids[parseInt(placeholderSplit[1])]] = currentNode;
            if (!treeWalker.nextNode()) {
              break;
            }
            continue;
          }

          // Then try to hydrate the comment node as a marker for the next text node
          const textNodeSplit = id.split("node-id");

          if (textNodeSplit.length > 1) {
            // For most text nodes, this should be text
            let next = currentNode.nextSibling;
            // remove the comment node
            currentNode.remove();

            let commentAfterText;
            let textNode;

            // If we are hydrating an empty text node, we may see two comment nodes in a row instead of a comment node, text node and then comment node
            if (next.nodeType === Node.COMMENT_NODE) {
              const newText = next.parentElement.insertBefore(
                document.createTextNode(""),
                next
              );
              commentAfterText = next;
              textNode = newText;
            } else {
              // The node after text should be a comment node marking the end of the text node
              textNode = next;
              commentAfterText = textNode.nextSibling;
            }
            treeWalker.currentNode = commentAfterText;
            this.nodes[ids[parseInt(textNodeSplit[1])]] = textNode;
            let exit = !treeWalker.nextNode();
            // remove the comment node after the text node
            commentAfterText.remove();
            if (exit) {
              break;
            }
            continue;
          }
        }
        if (!treeWalker.nextNode()) {
          break;
        }
      }
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
