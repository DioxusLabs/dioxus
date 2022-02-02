export function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.rpc.call("initialize");
  }
}

declare global {
  interface Window {
    interpreter: Interpreter;
    rpc: { call: (method: string, args?: any) => void };
  }
}


export class Interpreter {
  root: Element;
  stack: Element[];
  listeners: { [key: string]: number };
  handlers: { [key: string]: (evt: Event) => void };
  lastNodeWasText: boolean;
  nodes: Element[];


  constructor(root: Element) {
    this.root = root;
    this.stack = [root];
    this.listeners = {};
    this.handlers = {};
    this.lastNodeWasText = false;
    this.nodes = [root];
  }

  top() {
    return this.stack[this.stack.length - 1];
  }

  pop() {
    return this.stack.pop();
  }

  PushRoot(root: number) {
    const node = this.nodes[root];
    this.stack.push(node);
  }

  AppendChildren(many: number) {
    let root = this.stack[this.stack.length - (1 + many)];

    let to_add = this.stack.splice(this.stack.length - many);

    for (let i = 0; i < many; i++) {
      root.appendChild(to_add[i]);
    }
  }

  ReplaceWith(root_id: number, m: number) {
    let root = this.nodes[root_id] as Element;
    let els = this.stack.splice(this.stack.length - m);

    root.replaceWith(...els);
  }

  InsertAfter(root: number, n: number) {
    let old = this.nodes[root] as Element;
    let new_nodes = this.stack.splice(this.stack.length - n);
    old.after(...new_nodes);
  }

  InsertBefore(root: number, n: number) {
    let old = this.nodes[root] as Element;
    let new_nodes = this.stack.splice(this.stack.length - n);
    old.before(...new_nodes);
  }

  Remove(root: number) {
    let node = this.nodes[root] as Element;
    if (node !== undefined) {
      node.remove();
    }
  }

  CreateTextNode(text: string, root: number) {
    // todo: make it so the types are okay
    const node = document.createTextNode(text) as any as Element;
    this.nodes[root] = node;
    this.stack.push(node);
  }

  CreateElement(tag: string, root: number) {
    const el = document.createElement(tag);
    // el.setAttribute("data-dioxus-id", `${root}`);

    this.nodes[root] = el;
    this.stack.push(el);
  }

  CreateElementNs(tag: string, root: number, ns: string) {
    let el = document.createElementNS(ns, tag);
    this.stack.push(el);
    this.nodes[root] = el;
  }

  CreatePlaceholder(root: number) {
    let el = document.createElement("pre");
    el.hidden = true;
    this.stack.push(el);
    this.nodes[root] = el;
  }

  NewEventListener(event_name: string, root: number, handler: (evt: Event) => void) {
    const element = this.nodes[root];
    element.setAttribute("data-dioxus-id", `${root}`);

    if (this.listeners[event_name] === undefined) {
      this.listeners[event_name] = 0;
      this.handlers[event_name] = handler;
      this.root.addEventListener(event_name, handler);
    } else {
      this.listeners[event_name]++;
    }
  }

  RemoveEventListener(root: number, event_name: string) {
    const element = this.nodes[root];
    element.removeAttribute(`data-dioxus-id`);

    this.listeners[event_name]--;

    if (this.listeners[event_name] === 0) {
      this.root.removeEventListener(event_name, this.handlers[event_name]);
      delete this.listeners[event_name];
      delete this.handlers[event_name];
    }
  }


  SetText(root: number, text: string) {
    this.nodes[root].textContent = text;
  }

  SetAttribute(root: number, field: string, value: string, ns: string | undefined) {
    const name = field;
    const node = this.nodes[root];

    if (ns == "style") {

      // @ts-ignore
      (node as HTMLElement).style[name] = value;

    } else if (ns != null || ns != undefined) {
      node.setAttributeNS(ns, name, value);
    } else {
      switch (name) {
        case "value":
          if (value != (node as HTMLInputElement).value) {
            (node as HTMLInputElement).value = value;
          }
          break;
        case "checked":
          (node as HTMLInputElement).checked = value === "true";
          break;
        case "selected":
          (node as HTMLOptionElement).selected = value === "true";
          break;
        case "dangerous_inner_html":
          node.innerHTML = value;
          break;
        default:
          // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
          if (value == "false" && bool_attrs.hasOwnProperty(name)) {
            node.removeAttribute(name);
          } else {
            node.setAttribute(name, value);
          }
      }
    }
  }
  RemoveAttribute(root: number, name: string) {

    const node = this.nodes[root];
    node.removeAttribute(name);

    if (name === "value") {
      (node as HTMLInputElement).value = "";
    }

    if (name === "checked") {
      (node as HTMLInputElement).checked = false;
    }

    if (name === "selected") {
      (node as HTMLOptionElement).selected = false;
    }
  }

  handleEdits(edits: DomEdit[]) {
    this.stack.push(this.root);

    for (let edit of edits) {
      this.handleEdit(edit);
    }
  }

  handleEdit(edit: DomEdit) {
    switch (edit.type) {
      case "PushRoot":
        this.PushRoot(edit.root);
        break;
      case "AppendChildren":
        this.AppendChildren(edit.many);
        break;
      case "ReplaceWith":
        this.ReplaceWith(edit.root, edit.m);
        break;
      case "InsertAfter":
        this.InsertAfter(edit.root, edit.n);
        break;
      case "InsertBefore":
        this.InsertBefore(edit.root, edit.n);
        break;
      case "Remove":
        this.Remove(edit.root);
        break;
      case "CreateTextNode":
        this.CreateTextNode(edit.text, edit.root);
        break;
      case "CreateElement":
        this.CreateElement(edit.tag, edit.root);
        break;
      case "CreateElementNs":
        this.CreateElementNs(edit.tag, edit.root, edit.ns);
        break;
      case "CreatePlaceholder":
        this.CreatePlaceholder(edit.root);
        break;
      case "RemoveEventListener":
        this.RemoveEventListener(edit.root, edit.event_name);
        break;
      case "NewEventListener":


        // this handler is only provided on desktop implementations since this
        // method is not used by the web implementation
        let handler = (event: Event) => {
          let target = event.target as Element | null;

          if (target != null) {
            let realId = target.getAttribute(`data-dioxus-id`);
            let shouldPreventDefault = target.getAttribute(`dioxus-prevent-default`);

            if (event.type == "click") {
              event.preventDefault();
              if (shouldPreventDefault !== `onclick`) {
                if (target.tagName == "A") {
                  const href = target.getAttribute("href")
                  if (href !== "" && href !== null && href !== undefined) {
                    window.rpc.call("browser_open", { href });
                  }
                }
              }
            }

            // walk the tree to find the real element
            while (realId == null && target.parentElement != null) {
              target = target.parentElement;
              realId = target.getAttribute(`data-dioxus-id`);
            }

            shouldPreventDefault = target.getAttribute(`dioxus-prevent-default`);

            let contents = serialize_event(event);

            if (shouldPreventDefault === `on${event.type}`) {
              event.preventDefault();
            }

            if (event.type == "submit") {
              event.preventDefault();
            }


            if (target.tagName == "FORM") {
              let formTarget = target as HTMLFormElement;
              for (let x = 0; x < formTarget.elements.length; x++) {
                let element = formTarget.elements[x];
                let name = element.getAttribute("name");
                if (name != null) {
                  if (element.getAttribute("type") == "checkbox") {
                    // @ts-ignore
                    contents.values[name] = element.checked ? "true" : "false";
                  } else {
                    // @ts-ignore
                    contents.values[name] = element.value ?? element.textContent;
                  }
                }
              }
            }

            if (realId == null) {
              return;
            }

            window.rpc.call("user_event", {
              event: (edit as NewEventListener).event_name,
              mounted_dom_id: parseInt(realId),
              contents: contents,
            });
          }
        };
        this.NewEventListener(edit.event_name, edit.root, handler);
        break;
      case "SetText":
        this.SetText(edit.root, edit.text);
        break;
      case "SetAttribute":
        this.SetAttribute(edit.root, edit.field, edit.value, edit.ns);
        break;
      case "RemoveAttribute":
        this.RemoveAttribute(edit.root, edit.name);
        break;
    }
  }
}



function serialize_event(event: Event) {
  switch (event.type) {
    case "copy":
    case "cut":
    case "past": {
      return {};
    }

    case "compositionend":
    case "compositionstart":
    case "compositionupdate": {
      let { data } = (event as CompositionEvent);
      return {
        data,
      };
    }

    case "keydown":
    case "keypress":
    case "keyup": {
      let {
        charCode,
        key,
        altKey,
        ctrlKey,
        metaKey,
        keyCode,
        shiftKey,
        location,
        repeat,
        which,
      } = (event as KeyboardEvent);

      return {
        char_code: charCode,
        key: key,
        alt_key: altKey,
        ctrl_key: ctrlKey,
        meta_key: metaKey,
        key_code: keyCode,
        shift_key: shiftKey,
        location: location,
        repeat: repeat,
        which: which,
        locale: "locale",
      };
    }

    case "focus":
    case "blur": {
      return {};
    }

    case "change": {
      let target = event.target as HTMLInputElement;
      let value;
      if (target.type === "checkbox" || target.type === "radio") {
        value = target.checked ? "true" : "false";
      } else {
        value = target.value ?? target.textContent;
      }

      return {
        value: value,
      };
    }

    case "input":
    case "invalid":
    case "reset":
    case "submit": {


      let target = event.target as HTMLFormElement;
      let value = target.value ?? target.textContent;

      if (target.type == "checkbox") {
        value = target.checked ? "true" : "false";
      }

      return {
        value: value,
        values: {}
      };
    }

    case "click":
    case "contextmenu":
    case "doubleclick":
    case "drag":
    case "dragend":
    case "dragenter":
    case "dragexit":
    case "dragleave":
    case "dragover":
    case "dragstart":
    case "drop":
    case "mousedown":
    case "mouseenter":
    case "mouseleave":
    case "mousemove":
    case "mouseout":
    case "mouseover":
    case "mouseup": {
      const {
        altKey,
        button,
        buttons,
        clientX,
        clientY,
        ctrlKey,
        metaKey,
        pageX,
        pageY,
        screenX,
        screenY,
        shiftKey,
      } = event as MouseEvent;

      return {
        alt_key: altKey,
        button: button,
        buttons: buttons,
        client_x: clientX,
        client_y: clientY,
        ctrl_key: ctrlKey,
        meta_key: metaKey,
        page_x: pageX,
        page_y: pageY,
        screen_x: screenX,
        screen_y: screenY,
        shift_key: shiftKey,
      };
    }

    case "pointerdown":
    case "pointermove":
    case "pointerup":
    case "pointercancel":
    case "gotpointercapture":
    case "lostpointercapture":
    case "pointerenter":
    case "pointerleave":
    case "pointerover":
    case "pointerout": {
      const {
        altKey,
        button,
        buttons,
        clientX,
        clientY,
        ctrlKey,
        metaKey,
        pageX,
        pageY,
        screenX,
        screenY,
        shiftKey,
        pointerId,
        width,
        height,
        pressure,
        tangentialPressure,
        tiltX,
        tiltY,
        twist,
        pointerType,
        isPrimary,
      } = event as PointerEvent;
      return {
        alt_key: altKey,
        button: button,
        buttons: buttons,
        client_x: clientX,
        client_y: clientY,
        ctrl_key: ctrlKey,
        meta_key: metaKey,
        page_x: pageX,
        page_y: pageY,
        screen_x: screenX,
        screen_y: screenY,
        shift_key: shiftKey,
        pointer_id: pointerId,
        width: width,
        height: height,
        pressure: pressure,
        tangential_pressure: tangentialPressure,
        tilt_x: tiltX,
        tilt_y: tiltY,
        twist: twist,
        pointer_type: pointerType,
        is_primary: isPrimary,
      };
    }

    case "select": {
      return {};
    }

    case "touchcancel":
    case "touchend":
    case "touchmove":
    case "touchstart": {
      const {
        altKey,
        ctrlKey,
        metaKey,
        shiftKey,
      } = event as TouchEvent;
      return {
        // changed_touches: event.changedTouches,
        // target_touches: event.targetTouches,
        // touches: event.touches,
        alt_key: altKey,
        ctrl_key: ctrlKey,
        meta_key: metaKey,
        shift_key: shiftKey,
      };
    }

    case "scroll": {
      return {};
    }

    case "wheel": {
      const {
        deltaX,
        deltaY,
        deltaZ,
        deltaMode,
      } = event as WheelEvent;
      return {
        delta_x: deltaX,
        delta_y: deltaY,
        delta_z: deltaZ,
        delta_mode: deltaMode,
      };
    }

    case "animationstart":
    case "animationend":
    case "animationiteration": {
      const {
        animationName,
        elapsedTime,
        pseudoElement,
      } = event as AnimationEvent;
      return {
        animation_name: animationName,
        elapsed_time: elapsedTime,
        pseudo_element: pseudoElement,
      };
    }

    case "transitionend": {
      const {
        propertyName,
        elapsedTime,
        pseudoElement,
      } = event as TransitionEvent;
      return {
        property_name: propertyName,
        elapsed_time: elapsedTime,
        pseudo_element: pseudoElement,
      };
    }

    case "abort":
    case "canplay":
    case "canplaythrough":
    case "durationchange":
    case "emptied":
    case "encrypted":
    case "ended":
    case "error":
    case "loadeddata":
    case "loadedmetadata":
    case "loadstart":
    case "pause":
    case "play":
    case "playing":
    case "progress":
    case "ratechange":
    case "seeked":
    case "seeking":
    case "stalled":
    case "suspend":
    case "timeupdate":
    case "volumechange":
    case "waiting": {
      return {};
    }

    case "toggle": {
      return {};
    }

    default: {
      return {};
    }
  }
}

const bool_attrs = {
  allowfullscreen: true,
  allowpaymentrequest: true,
  async: true,
  autofocus: true,
  autoplay: true,
  checked: true,
  controls: true,
  default: true,
  defer: true,
  disabled: true,
  formnovalidate: true,
  hidden: true,
  ismap: true,
  itemscope: true,
  loop: true,
  multiple: true,
  muted: true,
  nomodule: true,
  novalidate: true,
  open: true,
  playsinline: true,
  readonly: true,
  required: true,
  reversed: true,
  selected: true,
  truespeed: true,
};

type PushRoot = { type: "PushRoot", root: number };
type AppendChildren = { type: "AppendChildren", many: number };
type ReplaceWith = { type: "ReplaceWith", root: number, m: number };
type InsertAfter = { type: "InsertAfter", root: number, n: number };
type InsertBefore = { type: "InsertBefore", root: number, n: number };
type Remove = { type: "Remove", root: number };
type CreateTextNode = { type: "CreateTextNode", text: string, root: number };
type CreateElement = { type: "CreateElement", tag: string, root: number };
type CreateElementNs = { type: "CreateElementNs", tag: string, root: number, ns: string };
type CreatePlaceholder = { type: "CreatePlaceholder", root: number };
type NewEventListener = { type: "NewEventListener", root: number, event_name: string, scope: number };
type RemoveEventListener = { type: "RemoveEventListener", event_name: string, scope: number, root: number };
type SetText = { type: "SetText", root: number, text: string };
type SetAttribute = { type: "SetAttribute", root: number, field: string, value: string, ns: string | undefined };
type RemoveAttribute = { type: "RemoveAttribute", root: number, name: string };

type DomEdit =
  PushRoot |
  AppendChildren |
  ReplaceWith |
  InsertAfter |
  InsertBefore |
  Remove |
  CreateTextNode |
  CreateElement |
  CreateElementNs |
  CreatePlaceholder |
  NewEventListener |
  RemoveEventListener |
  SetText |
  SetAttribute |
  RemoveAttribute;
