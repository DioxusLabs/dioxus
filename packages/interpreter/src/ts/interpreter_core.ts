// The root interpreter class that holds state about the mapping between DOM and VirtualDom
// This always lives in the JS side of things, and is extended by the native and web interpreters

export class Interpreter {
  // non bubbling events listen at the element the listener was created at
  global: {
    [key: string]: { active: number, callback: EventListener }
  };
  // bubbling events can listen at the root element
  local: {
    [key: string]: {
      [key: string]: EventListener
    }
  };

  root: HTMLElement;
  handler: EventListener;
  nodes: Node[];
  stack: Node[];
  templates: {
    [key: string]: Node[]
  };

  constructor(root: HTMLElement, handler: EventListener) {
    this.root = root;
    this.nodes = [root];
    this.stack = [root];
    this.global = {};
    this.local = {};
    this.handler = handler;
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
      this.root.removeEventListener(event_name, this.global[event_name].callback);
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

  getNode(id: number): Node {
    return this.nodes[id];
  }

  appendChildren(id: number, many: number) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0; k < many; k++) {
      root.appendChild(els[k]);
    }
  }
}

