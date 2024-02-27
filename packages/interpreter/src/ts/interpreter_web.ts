// The JS<->Rust bridge
// This file is thin, suitable just for the web
// If you want the more full-featured intrepreter, look at the native interpreter which extends this with additional functionality
//
// We're using sledgehammer directly

import { Interpreter, m } from "./interpreter_core";

export class WebInterpreter extends Interpreter {
  constructor(root: Element, handler: EventListener) {
    super(root, handler);
  }

  LoadChild(ptr: number, len: number): Node {
    // iterate through each number and get that child
    let node = this.stack[this.stack.length - 1] as Node;
    let ptr_end = ptr + len;

    for (; ptr < ptr_end; ptr++) {
      let end = m.getUint8(ptr);
      for (node = node.firstChild; end > 0; end--) {
        node = node.nextSibling;
      }
    }

    return node;
  }

  saveTemplate(nodes: Element[], tmpl_id: string) {
    this.templates[tmpl_id] = nodes;
  }

  hydrateRoot(ids: { [key: number]: number }) {
    const hydrateNodes = document.querySelectorAll('[data-node-hydration]');

    for (let i = 0; i < hydrateNodes.length; i++) {
      const hydrateNode = hydrateNodes[i];
      const hydration = hydrateNode.getAttribute('data-node-hydration');
      const split = hydration!.split(',');
      const id = ids[parseInt(split[0])];

      this.nodes[id] = hydrateNode;

      if (split.length > 1) {
        // @ts-ignore
        hydrateNode.listening = split.length - 1;
        hydrateNode.setAttribute('data-dioxus-id', id.toString());
        for (let j = 1; j < split.length; j++) {
          const listener = split[j];
          const split2 = listener.split(':');
          const event_name = split2[0];
          const bubbles = split2[1] === '1';
          this.createListener(event_name, hydrateNode, bubbles);
        }
      }
    }
    const treeWalker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_COMMENT,
    );
    let currentNode = treeWalker.nextNode();
    while (currentNode) {
      const id = currentNode.textContent!;
      const split = id.split('node-id');
      if (split.length > 1) {
        this.nodes[ids[parseInt(split[1])]] = currentNode.nextSibling as Element;
      }
      currentNode = treeWalker.nextNode();
    }
  }

  getNode(id: number): Element {
    return this.nodes[id];
  }

  appendChildren(id: number, many: number) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0; k < many; k++) {
      this.root.appendChild(els[k]);
    }
  }
}