// This file provides an extended variant of the interpreter used for desktop and liveview interaction
//
// This file lives on the renderer, not the host. It's basically a polyfill over functionality that the host can't
// provide since it doesn't have access to the dom.

import { BaseInterpreter, NodeId } from "./core";
import { SerializedEvent, serializeEvent, SerializedFileData, extractSerializedFormValues, SerializedFormObject } from "./serialize";

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
  edits: WebSocket;
  baseUri: string;
  eventsPath: string;
  headless: boolean;
  kickStylesheets: boolean;
  queuedBytes: ArrayBuffer[] = [];

  // eventually we want to remove liveview and build it into the server-side-events of fullstack
  // however, for now we need to support it since WebSockets in fullstack doesn't exist yet
  liveview: boolean;

  constructor(baseUri: string, headless: boolean) {
    super();
    this.baseUri = baseUri;
    this.eventsPath = `${baseUri}/__events`;
    this.kickStylesheets = false;
    this.headless = headless;
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
          // Handle file inputs specifically, since we want to get the real file inputs from the native
          // dialog and then set the form inputs accordingly.
          if (
            target instanceof HTMLInputElement &&
            target.getAttribute("type") === "file"
          ) {
            // Send a message to the host to open the file dialog if the target is a file input and has a dioxus id attached to it
            event.preventDefault();

            const contents = serializeEvent(event, target);

            const target_name = target.getAttribute("name") || "";

            let requestData = {
              event: "change&input",
              accept: target.getAttribute("accept"),
              directory: target.getAttribute("webkitdirectory") === "true",
              multiple: target.hasAttribute("multiple"),
              target: target_id,
              bubbles: event.bubbles,
              target_name,
              values: contents.values,
            };

            this.fetchAgainstHost("__file_dialog", requestData).then(response => response.json()).then(resp => {
              const formObjects: SerializedFormObject[] = resp.values;

              // Create a new DataTransfer to hold the files
              const dataTransfer = new DataTransfer();

              // We name the file the path, so we can just use the path as the name later on.
              for (let formObject of formObjects) {
                if (formObject.key == target_name && formObject.file != null) {
                  const file = new File([], formObject.file.path, {
                    type: formObject.file.content_type,
                    lastModified: formObject.file.last_modified,
                  });
                  dataTransfer.items.add(file);
                }
              }

              // Set the files on the input
              target.files = dataTransfer.files;

              let body = {
                data: contents,
                element: target_id,
                bubbles: event.bubbles,
              };

              // And then dispatch the actual event against the dom
              contents.values = formObjects;
              this.sendSerializedEvent({
                ...body,
                name: "input",
              });
              this.sendSerializedEvent({
                ...body,
                name: "change",
              });
            });

            return;
          }

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

  fetchAgainstHost(path: string, data: { [key: string]: any }): Promise<Response> {
    let encoded_data = new TextEncoder().encode(JSON.stringify(data));
    const base64data = btoa(
      String.fromCharCode.apply(null, Array.from(encoded_data))
    );

    return fetch(`${this.baseUri}/${path}`, {
      method: "GET",
      headers: {
        "x-dioxus-data": base64data
      }
    });
  }

  sendIpcMessage(method: string, params = {}) {
    const body = JSON.stringify({ method, params });
    this.ipc.postMessage(body);
  }

  scrollTo(id: NodeId, options: ScrollIntoViewOptions): boolean {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      node.scrollIntoView(options);
      return true;
    }
    return false;
  }

  scroll(id: NodeId, x: number, y: number, behavior: ScrollBehavior): boolean {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      node.scroll({ top: y, left: x, behavior });
      return true;
    }
    return false;
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

  // Windows drag-n-drop fix code. Called by wry drag-n-drop handler over the event loop.
  handleWindowsDragDrop() {
    // @ts-ignore
    if (window.dxDragLastElement) {
      const dragLeaveEvent = new DragEvent("dragleave", {
        bubbles: true,
        cancelable: true,
      });

      // @ts-ignore
      window.dxDragLastElement.dispatchEvent(dragLeaveEvent);

      let data = new DataTransfer();

      // We need to mimic that there are actually files in this event for our native file engine to pick it up.
      const file = new File(["content"], "file.txt", { type: "text/plain" });
      data.items.add(file);

      const dragDropEvent = new DragEvent("drop", {
        bubbles: true,
        cancelable: true,
        dataTransfer: data,
      });

      // @ts-ignore
      window.dxDragLastElement.dispatchEvent(dragDropEvent);
      // @ts-ignore
      window.dxDragLastElement = null;
    }
  }

  handleWindowsDragOver(xPos: number, yPos: number) {
    const displayScaleFactor = window.devicePixelRatio || 1;
    xPos /= displayScaleFactor;
    yPos /= displayScaleFactor;
    const element = document.elementFromPoint(xPos, yPos);

    // @ts-ignore
    if (element != window.dxDragLastElement) {
      // @ts-ignore
      if (window.dxDragLastElement) {
        const dragLeaveEvent = new DragEvent("dragleave", {
          bubbles: true,
          cancelable: true,
        });
        // @ts-ignore
        window.dxDragLastElement.dispatchEvent(dragLeaveEvent);
      }

      const dragOverEvent = new DragEvent("dragover", {
        bubbles: true,
        cancelable: true,
      });
      element.dispatchEvent(dragOverEvent);
      // @ts-ignore
      window.dxDragLastElement = element;
    }
  }

  handleWindowsDragLeave() {
    // @ts-ignore
    if (window.dxDragLastElement) {
      const dragLeaveEvent = new DragEvent("dragleave", {
        bubbles: true,
        cancelable: true,
      });
      // @ts-ignore
      window.dxDragLastElement.dispatchEvent(dragLeaveEvent);
      // @ts-ignore
      window.dxDragLastElement = null;
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
    const element = getTargetId(target)!;
    const contents = serializeEvent(event, target);

    // Handle the event on the virtualdom and then preventDefault if it also preventsDefault
    // Some listeners
    let body = {
      name,
      data: contents,
      element,
      bubbles,
    };

    // liveview does not have synchronous event handling, so we need to send the event to the host
    if (
      this.liveview &&
      target instanceof HTMLInputElement &&
      (event.type === "change" || event.type === "input")
    ) {
      if (target.getAttribute("type") === "file") {
        this.readFiles(target, contents, bubbles, element, name);
        return;
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
      this.sendIpcMessage("user_event", body);
    } else {
      // Run the event handler on the virtualdom
      return handleVirtualdomEventSync(this.eventsPath, JSON.stringify(body));
    }
  }

  handleClickNavigate(event: Event, target: Element) {
    // todo call prevent default if it's the right type of event
    if (!this.intercept_link_redirects) {
      return;
    }

    // If the target is an anchor tag, we want to intercept the click too, to prevent the browser from navigating
    let a_element = target.closest("a");
    if (a_element) {
      event.preventDefault();

      const href = a_element.getAttribute("href");
      if (href !== "" && href !== null && href !== undefined) {
        this.sendIpcMessage("browser_open", { href })
      }
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
  rafEdits(bytes: ArrayBuffer) {
    // In headless mode, the requestAnimationFrame callback is never called, so we need to run the bytes directly
    if (this.headless) {
      // @ts-ignore
      this.run_from_bytes(bytes);
      this.markEditsFinished();
    } else {
      this.enqueueBytes(bytes);
      requestAnimationFrame(() => {
        this.flushQueuedBytes();
        this.markEditsFinished();
      });
    }
  }

  waitForRequest(editsPath: string, required_server_key: string) {
    this.edits = new WebSocket(editsPath);
    // Only trust the websocket once it sends us the required server key
    let authenticated = false;
    // Reconnect if the websocket closes. This may happen on ios when the app is suspended
    // in the background: https://github.com/DioxusLabs/dioxus/issues/4374
    this.edits.onclose = () => {
      setTimeout(() => {
        // If the edits path has changed, we don't want to reconnect to the old one
        if (this.edits.url != editsPath) {
          return;
        }
        this.waitForRequest(editsPath, required_server_key);
      }, 100);
    };
    this.edits.onmessage = (event) => {
      const data = event.data;
      if (data instanceof Blob) {
        if (!authenticated) {
          return;
        }
        // If the data is a blob, we need to convert it to an ArrayBuffer
        data.arrayBuffer().then((buffer) => {
          this.rafEdits(buffer);
        });
      } else if (typeof data === "string") {
        if (data === required_server_key) {
          // If the data is the required server key, we can trust the websocket
          authenticated = true;
          return;
        }
      }
    };
  }

  markEditsFinished() {
    // Send an empty ArrayBuffer to the edits websocket to signal that the edits are finished
    // This is used to signal that the edits are done and the next request can be processed
    this.edits.send(new ArrayBuffer(0));
  }

  kickAllStylesheetsOnPage() {
    // If this function is being called and we have not explicitly set kickStylesheets to true, then we should
    // force kick the stylesheets, regardless if they have a dioxus attribute or not
    // This happens when any hotreload happens.
    let stylesheets = document.querySelectorAll("link[rel=stylesheet]");
    for (let i = 0; i < stylesheets.length; i++) {
      let sheet = stylesheets[i] as HTMLLinkElement;
      // Split up the url and add a extra random query param to force the browser to reload he asset
      const splitByQuery = sheet.href.split("?");
      let url = splitByQuery[0];
      let query = splitByQuery[1];
      if (!query) {
        query = "";
      }
      let queryParams = new URLSearchParams(query);
      // Delete the existing dx_force_reload entry if it exists
      queryParams.delete("dx_force_reload");
      // And add a new random dx_force_reload query param to force the browser to reload the asset
      queryParams.append("dx_force_reload", Math.random().toString());
      sheet.href = `${url}?${queryParams}`;
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

  // hack for android since we CANT SEND BODIES (because wry is using shouldInterceptRequest)
  //
  // https://issuetracker.google.com/issues/119844519
  // https://stackoverflow.com/questions/43273640/android-webviewclient-how-to-get-post-request-body
  // https://developer.android.com/reference/android/webkit/WebViewClient#shouldInterceptRequest(android.webkit.WebView,%20android.webkit.WebResourceRequest)
  //
  // the issue here isn't that big, tbh, but there's a small chance we lose the event due to header max size (16k per header, 32k max)
  const contents_bytes = new TextEncoder().encode(contents);
  const contents_base64 = btoa(String.fromCharCode.apply(null, contents_bytes));
  xhr.setRequestHeader("dioxus-data", contents_base64);
  xhr.send();

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
