// This file provides an extended variant of the interpreter used for desktop and liveview interaction
//
// This file lives on the renderer, not the host. It's basically a polyfill over functionality that the host can't
// provide since it doesn't have access to the dom.

import { BaseInterpreter, NodeId } from "./core";
import { SerializedEvent, serializeEvent } from "./serialize";

// okay so, we've got this JSChannel thing from sledgehammer, implicitly imported into our scope
// we want to extend it, and it technically extends base interpreter. To make typescript happy,
// we're going to bind the JSChannel_ object to the JSChannel object, and then extend it
var JSChannel_: typeof BaseInterpreter;

// @ts-ignore - this is coming from the host
if (RawInterpreter !== undefined && RawInterpreter !== null) {
  // @ts-ignore - this is coming from the host
  JSChannel_ = RawInterpreter;
}

export class NativeInterpreter extends JSChannel_ {
  intercept_link_redirects: boolean;
  ipc: any;
  editsPath: string;
  eventsPath: string;
  kickStylesheets: boolean;
  queuedBytes: ArrayBuffer[] = [];

  // eventually we want to remove liveview and build it into the server-side-events of fullstack
  // however, for now we need to support it since WebSockets in fullstack doesn't exist yet
  liveview: boolean;

  constructor(editsPath: string, eventsPath: string) {
    super();
    this.editsPath = editsPath;
    this.eventsPath = eventsPath;
    this.kickStylesheets = false;
  }

  initialize(root: HTMLElement): void {
    this.intercept_link_redirects = true;
    this.liveview = false;

    // attach an event listener on the body that prevents file drops from navigating
    // this is because the browser will try to navigate to the file if it's dropped on the window
    window.addEventListener(
      "dragover",
      function (e) {
        // // check which element is our target
        if (e.target instanceof Element && e.target.tagName != "INPUT") {
          e.preventDefault();
        }
      },
      false
    );

    window.addEventListener(
      "drop",
      function (e) {
        let target = e.target;

        if (!(target instanceof Element)) {
          return;
        }

        // Dropping a file on the window will navigate to the file, which we don't want
        e.preventDefault();
      },
      false
    );

    // attach a listener to the route that listens for clicks and prevents the default file dialog
    window.addEventListener("click", (event) => {
      const target = event.target;
      if (
        target instanceof HTMLInputElement &&
        target.getAttribute("type") === "file"
      ) {
        // Send a message to the host to open the file dialog if the target is a file input and has a dioxus id attached to it
        let target_id = getTargetId(target);
        if (target_id !== null) {
          const message = this.serializeIpcMessage("file_dialog", {
            event: "change&input",
            accept: target.getAttribute("accept"),
            directory: target.getAttribute("webkitdirectory") === "true",
            multiple: target.hasAttribute("multiple"),
            target: target_id,
            bubbles: event.bubbles,
          });
          this.ipc.postMessage(message);
          event.preventDefault();
        }
      }
    });

    // @ts-ignore - wry gives us this
    this.ipc = window.ipc;

    // make sure we pass the handler to the base interpreter
    const handler: EventListener = (event) =>
      this.handleEvent(event, event.type, true);

    super.initialize(root, handler);
  }

  serializeIpcMessage(method: string, params = {}) {
    return JSON.stringify({ method, params });
  }

  scrollTo(id: NodeId, behavior: ScrollBehavior) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      node.scrollIntoView({ behavior });
    }
  }

  getScrollHeight(id: NodeId): number | undefined {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      return node.scrollHeight;
    }
  }

  getScrollLeft(id: NodeId): number | undefined {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      return node.scrollLeft;
    }
  }

  getScrollTop(id: NodeId): number | undefined {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      return node.scrollTop;
    }
  }

  getScrollWidth(id: NodeId): number | undefined {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      return node.scrollWidth;
    }
  }

  getClientRect(
    id: NodeId
  ): { type: string; origin: number[]; size: number[] } | undefined {
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

  setFocus(id: NodeId, focus: boolean) {
    const node = this.nodes[id];

    if (node instanceof HTMLElement) {
      if (focus) {
        node.focus();
      } else {
        node.blur();
      }
    }
  }

  // ignore the fact the base interpreter uses ptr + len but we use array...
  // @ts-ignore
  loadChild(array: number[]) {
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

  appendChildren(id: NodeId, many: number) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);

    for (let k = 0; k < many; k++) {
      root.appendChild(els[k]);
    }
  }

  handleEvent(event: Event, name: string, bubbles: boolean) {
    const target = event.target!;
    const realId = getTargetId(target)!;
    const contents = serializeEvent(event, target);

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
    this.preventDefaults(event);

    // liveview does not have synchronous event handling, so we need to send the event to the host
    if (this.liveview) {
      // Okay, so the user might've requested some files to be read
      if (
        target instanceof HTMLInputElement &&
        (event.type === "change" || event.type === "input")
      ) {
        if (target.getAttribute("type") === "file") {
          this.readFiles(target, contents, bubbles, realId, name);
          return;
        }
      }
    }
    const response = this.sendSerializedEvent(body);
    // capture/prevent default of the event if the virtualdom wants to
    if (response) {
      if (response.preventDefault) {
        event.preventDefault();
      } else {
        // Attempt to intercept if the event is a click and the default action was not prevented
        if (target instanceof Element && event.type === "click") {
          this.handleClickNavigate(event, target);
        }
      }

      if (response.stopPropagation) {
        event.stopPropagation();
      }
    }
  }

  sendSerializedEvent(body: {
    name: string;
    element: number;
    data: any;
    bubbles: boolean;
  }): EventSyncResult | void {
    if (this.liveview) {
      const message = this.serializeIpcMessage("user_event", body);
      this.ipc.postMessage(message);
    } else {
      // Run the event handler on the virtualdom
      return handleVirtualdomEventSync(this.eventsPath, JSON.stringify(body));
    }
  }

  // This should:
  // - prevent form submissions from navigating
  // - prevent anchor tags from navigating
  // - prevent buttons from submitting forms
  // - let the virtualdom attempt to prevent the event
  preventDefaults(event: Event) {
    if (event.type === "submit") {
      event.preventDefault();
    }
  }

  handleClickNavigate(event: Event, target: Element) {
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

    const href = a_element.getAttribute("href");
    if (href !== "" && href !== null && href !== undefined) {
      this.ipc.postMessage(this.serializeIpcMessage("browser_open", { href }));
    }
  }

  enqueueBytes(bytes: ArrayBuffer) {
    this.queuedBytes.push(bytes);
  }

  flushQueuedBytes() {
    // drain the queuedBytes
    const byteArray = this.queuedBytes;
    this.queuedBytes = [];

    for (let bytes of byteArray) {
      // @ts-ignore
      this.run_from_bytes(bytes);
    }
  }

  // Run the edits the next animation frame
  rafEdits(headless: boolean, bytes: ArrayBuffer) {
    // In headless mode, the requestAnimationFrame callback is never called, so we need to run the bytes directly
    if (headless) {
      // @ts-ignore
      this.run_from_bytes(bytes);
      this.waitForRequest(headless);
    } else {
      this.enqueueBytes(bytes);
      requestAnimationFrame(() => {
        this.flushQueuedBytes();
        // With request animation frames, we use the next reqwest as a marker to know when the frame is done and it is safe to run effects
        this.waitForRequest(headless);
      });
    }
  }

  waitForRequest(headless: boolean) {
    fetch(new Request(this.editsPath))
      .then((response) => response.arrayBuffer())
      .then((bytes) => {
        this.rafEdits(headless, bytes);
      });
  }

  kickAllStylesheetsOnPage() {
    // If this function is being called and we have not explicitly set kickStylesheets to true, then we should
    // force kick the stylesheets, regardless if they have a dioxus attribute or not
    // This happens when any hotreload happens.
    let stylesheets = document.querySelectorAll("link[rel=stylesheet]");
    for (let i = 0; i < stylesheets.length; i++) {
      let sheet = stylesheets[i] as HTMLLinkElement;
      // Using `cache: reload` will force the browser to re-fetch the stylesheet and bust the cache
      fetch(sheet.href, { cache: "reload" }).then(() => {
        sheet.href = sheet.href + "?" + Math.random();
      });
    }
  }

  //  A liveview only function
  // Desktop will intercept the event before it hits this
  async readFiles(
    target: HTMLInputElement,
    contents: SerializedEvent,
    bubbles: boolean,
    realId: NodeId,
    name: string
  ) {
    let files = target.files!;
    let file_contents: { [name: string]: number[] } = {};

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      file_contents[file.name] = Array.from(
        new Uint8Array(await file.arrayBuffer())
      );
    }

    contents.files = { files: file_contents };

    const message = this.sendSerializedEvent({
      name: name,
      element: realId,
      data: contents,
      bubbles,
    });

    this.ipc.postMessage(message);
  }
}

type EventSyncResult = {
  preventDefault: boolean;
  stopPropagation: boolean;
};

// This function sends the event to the virtualdom and then waits for the virtualdom to process it
//
// However, it's not really suitable for liveview, because it's synchronous and will block the main thread
// We should definitely consider using a websocket if we want to block... or just not block on liveview
// Liveview is a little bit of a tricky beast
function handleVirtualdomEventSync(
  endpoint: string,
  contents: string
): EventSyncResult {
  // Handle the event on the virtualdom and then process whatever its output was
  const xhr = new XMLHttpRequest();

  // Serialize the event and send it to the custom protocol in the Rust side of things
  xhr.open("POST", endpoint, false);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.send(contents);

  // Deserialize the response, and then prevent the default/capture the event if the virtualdom wants to
  return JSON.parse(xhr.responseText);
}

function getTargetId(target: EventTarget): NodeId | null {
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

// function applyFileUpload() {
//   let inputs = document.querySelectorAll("input");
//   for (let input of inputs) {
//     if (!input.getAttribute("data-dioxus-file-listener")) {
//       // prevent file inputs from opening the file dialog on click
//       const type = input.getAttribute("type");
//       if (type === "file") {
//         input.setAttribute("data-dioxus-file-listener", true);
//         input.addEventListener("click", (event) => {
//           let target = event.target;
//           let target_id = find_real_id(target);
//           if (target_id !== null) {
//             const send = (event_name) => {
//               const message = window.interpreter.serializeIpcMessage("file_dialog", { accept: target.getAttribute("accept"), directory: target.getAttribute("webkitdirectory") === "true", multiple: target.hasAttribute("multiple"), target: parseInt(target_id), bubbles: event_bubbles(event_name), event: event_name });
//               window.ipc.postMessage(message);
//             };
//             send("change&input");
//           }
//           event.preventDefault();
//         });
//       }
//     }
// }
