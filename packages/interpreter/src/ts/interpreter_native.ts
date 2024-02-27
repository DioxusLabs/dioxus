// This file provides an extended variant of the interpreter used for desktop and liveview interaction
//
//

import { Interpreter } from "./interpreter_core";

export class NativeInterpreter extends Interpreter {
  intercept_link_redirects: boolean;

  constructor(root: Element) {
    super(root, (event) => handler(event, this, event.type, true));
  }

  serializeIpcMessage(method: string, params = {}) {
    return JSON.stringify({ method, params });
  }

  scrollTo(id: number, behavior: ScrollBehavior) {
    const node = this.nodes[id];

    if (!(node instanceof HTMLElement)) {
      return false;
    }

    node.scrollIntoView({
      behavior: behavior,
    });

    return true;
  }

  getClientRect(id: number) {
    const node = this.nodes[id];
    if (!node) {
      return;
    }
    const rect = node.getBoundingClientRect();
    return {
      type: "GetClientRect",
      origin: [rect.x, rect.y],
      size: [rect.width, rect.height],
    };
  }

  setFocus(id: number, focus: boolean) {
    const node = this.nodes[id];

    if (!(node instanceof HTMLElement)) {
      return false;
    }

    if (focus) {
      node.focus();
    } else {
      node.blur();
    }

    return true;
  }

  LoadChild(array: number[]) {
    // iterate through each number and get that child
    let node = this.stack[this.stack.length - 1] as Node;

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
}




function handler(event: Event, interpreter: NativeInterpreter, name: string, bubbles: boolean) {
  const target = event.target!;
  const realId = target_id(target)!;
  let contents = serializeEvent(event);

  if (target instanceof HTMLFormElement && (event.type === "submit" || event.type === "input")) {
    const formData = new FormData(target);

    for (let name of formData.keys()) {
      const fieldType = target.elements[name].type;

      switch (fieldType) {
        case "select-multiple":
          contents.values[name] = formData.getAll(name);
          break;

        // add cases for fieldTypes that can hold multiple values here
        default:
          contents.values[name] = formData.get(name);
          break;
      }
    }
  }

  if (target instanceof HTMLSelectElement && (event.type === "input" || event.type === "change")) {
    const selectData = target.options;
    contents.values["options"] = [];
    for (let i = 0; i < selectData.length; i++) {
      let option = selectData[i];
      if (option.selected) {
        contents.values["options"].push(option.value.toString());
      }
    }
  }

  // If there's files to read
  if (target instanceof HTMLInputElement && (event.type === "change" || event.type === "input")) {
    if (target.getAttribute("type") === "file") {
      read_files(target, contents, bubbles, realId, name);
      return;
    }
  }

  prevents_default(event, target);


  // Handle the event on the virtualdom and then process whatever its output was
  let body = {
    name: name,
    data: serializeEvent(event),
    element: parseInt(realId),
    bubbles,
  };

  if (waitForVirtualDomPreventDefault(JSON.stringify(body))) {
    event.preventDefault();
  }
}

export function waitForVirtualDomPreventDefault(contents: string): boolean {


  // Handle the event on the virtualdom and then process whatever its output was
  const xhr = new XMLHttpRequest();

  // Serialize the event and send it to the custom protocol in the Rust side of things
  xhr.timeout = 1000;
  xhr.open("GET", "/handle/event.please", false);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.send(contents);

  // Deserialize the response, and then prevent the default/capture the event if the virtualdom wants to
  return JSON.parse(xhr.responseText).preventDefault;
}


async function read_files(target: HTMLInputElement, contents, bubbles, realId, name) {
  let files = target.files!;
  let file_contents: { [name: string]: number[] } = {};

  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    file_contents[file.name] = Array.from(
      new Uint8Array(await file.arrayBuffer())
    );
  }

  contents.files = { files: file_contents };

  if (realId === null) {
    return;
  }

  const message = window.interpreter.serializeIpcMessage("user_event", {
    name: name,
    element: parseInt(realId),
    data: contents,
    bubbles,
  });

  window.ipc.postMessage(message);
}


export function target_id(target: EventTarget): string | null {
  if (!(target instanceof Element)) {
    return null;
  }

  function find_real_id(target: Element): string | null {
    let realId = target.getAttribute(`data-dioxus-id`);

    // walk the tree to find the real element
    while (realId == null) {
      // we've reached the root we don't want to send an event
      if (target.parentElement === null) {
        return null;
      }

      target = target.parentElement;
      if (target instanceof Element) {
        realId = target.getAttribute(`data-dioxus-id`);
      }
    }

    return realId;
  }

  return find_real_id(target);
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
    this.preventFormNavigate(event, target);
  }
}

preventFormNavigate(event: Event, target: Element) {
  // todo call prevent default if it's the right type of event
  if (!this.intercept_link_redirects) {
    return;
  }

  // prevent buttons in forms from submitting the form
  if (target.tagName === "BUTTON") { //  && event.type == "submit"
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
      window.ipc.postMessage(
        window.interpreter.serializeIpcMessage("browser_open", { href })
      );
    }
  }
}
