// This file provides an extended variant of the interpreter used for desktop and liveview interaction
//
// This file lives on the renderer, not the host. It's basically a polyfill over functionality that the host can't
// provide since it doesn't have access to the dom.

import { retriveValues } from "./form";
import { Interpreter } from "./interpreter_core";
import { SerializedEvent, serializeEvent } from "./serialize";

export class NativeInterpreter extends Interpreter {
  intercept_link_redirects: boolean;
  ipc: any;

  // eventually we want to remove liveview and build it into the server-side-events of fullstack
  // however, for now we need to support it since SSE in fullstack doesn't exist yet
  liveview: boolean;

  constructor(root: HTMLElement) {
    super(root, (event) => this.handleEvent(event, event.type, true));
    this.intercept_link_redirects = true;
    this.liveview = false;

    // @ts-ignore - wry gives us this
    this.ipc = window.ipc;
  }

  serializeIpcMessage(method: string, params = {}) {
    return JSON.stringify({ method, params });
  }

  scrollTo(id: number, behavior: ScrollBehavior) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      node.scrollIntoView({ behavior });
    }
  }

  getClientRect(id: number) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      const rect = node.getBoundingClientRect();
      return {
        type: "GetClientRect",
        origin: [rect.x, rect.y],
        size: [rect.width, rect.height],
      };
    }
  }

  setFocus(id: number, focus: boolean) {
    const node = this.nodes[id];

    if (node instanceof HTMLElement) {
      if (focus) {
        node.focus();
      } else {
        node.blur();
      }
    }
  }

  LoadChild(array: number[]) {
    // iterate through each number and get that child
    let node = this.stack[this.stack.length - 1];

    for (let i = 0; i < array.length; i++) {
      let end = array[i];
      for (node = node.firstChild; end > 0; end--) {
        node = node.nextSibling;
      }
    }

    return node;
  }

  AppendChildren(id: number, many: number) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);

    for (let k = 0; k < many; k++) {
      root.appendChild(els[k]);
    }
  }

  handleEvent(event: Event, name: string, bubbles: boolean) {
    const target = event.target!;
    const realId = targetId(target)!;
    const contents = serializeEvent(event);

    // Attempt to retrive the values from the form and inputs
    if (target instanceof HTMLElement) {
      contents.values = retriveValues(event, target);
    }

    // Handle the event on the virtualdom and then preventDefault if it also preventsDefault
    // Some listeners
    let body = {
      name: name,
      data: contents,
      element: realId,
      bubbles,
    };

    // Run any prevent defaults the user might've set
    // This is to support the prevent_default: "onclick" attribute that dioxus has had for a while, but is not necessary
    // now that we expose preventDefault to the virtualdom on desktop
    // Liveview will still need to use this
    this.preventDefaults(event, target);


    // liveview does not have syncronous event handling, so we need to send the event to the host
    if (this.liveview) {
      // Okay, so the user might've requested some files to be read
      if (target instanceof HTMLInputElement && (event.type === "change" || event.type === "input")) {
        if (target.getAttribute("type") === "file") {
          this.readFiles(target, contents, bubbles, realId, name);
        }
      }
    } else {

      // Run the event handler on the virtualdom
      // capture/prevent default of the event if the virtualdom wants to
      const res = handleVirtualdomEventSync(JSON.stringify(body));

      if (res.preventDefault) {
        event.preventDefault();
      }

      if (res.stopPropagation) {
        event.stopPropagation();
      }
    }
  }

  async readFiles(target: HTMLInputElement, contents: SerializedEvent, bubbles: boolean, realId: number, name: string) {
    let files = target.files!;
    let file_contents: { [name: string]: number[] } = {};

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      file_contents[file.name] = Array.from(
        new Uint8Array(await file.arrayBuffer())
      );
    }

    contents.files = { files: file_contents };

    const message = this.serializeIpcMessage("user_event", {
      name: name,
      element: realId,
      data: contents,
      bubbles,
    });

    this.ipc.postMessage(message);
  }


  // This should:
  // - prevent form submissions from navigating
  // - prevent anchor tags from navigating
  // - prevent buttons from submitting forms
  // - let the virtualdom attempt to prevent the event
  preventDefaults(event: Event, target: EventTarget) {
    let preventDefaultRequests: string | null = null;

    // Some events can be triggered on text nodes, which don't have attributes
    if (target instanceof Element) {
      preventDefaultRequests = target.getAttribute(`dioxus-prevent-default`);
    }

    if (preventDefaultRequests && preventDefaultRequests.includes(`on${event.type}`)) {
      event.preventDefault();
    }

    if (event.type === "submit") {
      event.preventDefault();
    }

    // Attempt to intercept if the event is a click
    if (target instanceof Element && event.type === "click") {
      this.handleClickNavigate(event, target, preventDefaultRequests);
    }
  }

  handleClickNavigate(event: Event, target: Element, preventDefaultRequests: string) {
    // todo call prevent default if it's the right type of event
    if (!this.intercept_link_redirects) {
      return;
    }

    // prevent buttons in forms from submitting the form
    if (target.tagName === "BUTTON" && event.type == "submit") {
      event.preventDefault();
    }

    // If the target is an anchor tag, we want to intercept the click too, to prevent the browser from navigating
    let a_element = target.closest("a");
    if (a_element == null) {
      return;
    }

    event.preventDefault();

    let elementShouldPreventDefault =
      preventDefaultRequests && preventDefaultRequests.includes(`onclick`);

    let aElementShouldPreventDefault = a_element.getAttribute(
      `dioxus-prevent-default`
    );

    let linkShouldPreventDefault =
      aElementShouldPreventDefault &&
      aElementShouldPreventDefault.includes(`onclick`);

    if (!elementShouldPreventDefault && !linkShouldPreventDefault) {
      const href = a_element.getAttribute("href");
      if (href !== "" && href !== null && href !== undefined) {
        this.ipc.postMessage(
          this.serializeIpcMessage("browser_open", { href })
        );
      }
    }
  }
}

type EventSyncResult = {
  preventDefault: boolean;
  stopPropagation: boolean;
  stopImmediatePropagation: boolean;
  filesRequested: boolean;
};

// This function sends the event to the virtualdom and then waits for the virtualdom to process it
//
// However, it's not really suitable for liveview, because it's synchronous and will block the main thread
// We should definitely consider using a websocket if we want to block... or just not block on liveview
// Liveview is a little bit of a tricky beast
function handleVirtualdomEventSync(contents: string): EventSyncResult {
  // Handle the event on the virtualdom and then process whatever its output was
  const xhr = new XMLHttpRequest();

  // Serialize the event and send it to the custom protocol in the Rust side of things
  xhr.timeout = 1000;
  xhr.open("GET", "/handle/event.please", false);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.send(contents);

  // Deserialize the response, and then prevent the default/capture the event if the virtualdom wants to
  return JSON.parse(xhr.responseText);
}

function targetId(target: EventTarget): number | null {
  // Ensure that the target is a node, sometimes it's nota
  if (!(target instanceof Node)) {
    return null;
  }

  let ourTarget = target;
  let realId = null;

  while (realId == null) {
    if (ourTarget === null) {
      return null;
    }

    if (ourTarget instanceof Element) {
      realId = ourTarget.getAttribute(`data-dioxus-id`);
    }

    ourTarget = ourTarget.parentNode;
  }

  return parseInt(realId);
}
