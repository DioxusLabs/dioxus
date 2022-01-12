

function serialize_event(event: Event) {
  switch (event.type) {
    case "copy":
    case "cut":
    case "past":
      return {};

    case "compositionend":
    case "compositionstart":
    case "compositionupdate":
      let { data } = (event as CompositionEvent);
      return {
        data,
      };

    case "keydown":
    case "keypress":
    case "keyup":
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

    case "focus":
    case "blur":
      return {};

    case "change":
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

    case "select":
      return {};

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

    case "scroll":
      return {};
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
    case "waiting":
      return {};

    case "toggle":
      return {};

    default:
      return {};
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

class Interpreter {
  root: Element;
  stack: Element[];
  listeners: { [key: string]: (event: Event) => void };
  lastNodeWasText: boolean;
  nodes: Element[];


  constructor(root: Element) {
    this.root = root;
    this.stack = [root];
    this.listeners = {
    };
    this.lastNodeWasText = false;
    this.nodes = [root];
  }

  top() {
    return this.stack[this.stack.length - 1];
  }

  pop() {
    return this.stack.pop();
  }

  PushRoot(edit: PushRoot) {
    const id = edit.root;
    const node = this.nodes[id];
    this.stack.push(node);
  }

  AppendChildren(edit: AppendChildren) {
    let root = this.stack[this.stack.length - (1 + edit.many)];

    let to_add = this.stack.splice(this.stack.length - edit.many);

    for (let i = 0; i < edit.many; i++) {
      root.appendChild(to_add[i]);
    }
  }

  ReplaceWith(edit: ReplaceWith) {
    let root = this.nodes[edit.root] as Element;
    let els = this.stack.splice(this.stack.length - edit.m);

    root.replaceWith(...els);
  }

  InsertAfter(edit: InsertAfter) {
    let old = this.nodes[edit.root] as Element;
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    old.after(...new_nodes);
  }

  InsertBefore(edit: InsertBefore) {
    let old = this.nodes[edit.root] as Element;
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    old.before(...new_nodes);
  }

  Remove(edit: Remove) {
    let node = this.nodes[edit.root] as Element;
    if (node !== undefined) {
      node.remove();
    }
  }

  CreateTextNode(edit: CreateTextNode) {
    // todo: make it so the types are okay
    const node = document.createTextNode(edit.text) as any as Element;
    this.nodes[edit.root] = node;
    this.stack.push(node);
  }

  CreateElement(edit: CreateElement) {
    const el = document.createElement(edit.tag);
    el.setAttribute("dioxus-id", `${edit.root}`);

    this.nodes[edit.root] = el;
    this.stack.push(el);
  }

  CreateElementNs(edit: CreateElementNs) {
    let el = document.createElementNS(edit.ns, edit.tag);
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  CreatePlaceholder(edit: CreatePlaceholder) {
    let el = document.createElement("pre");
    el.hidden = true;
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  RemoveEventListener(edit: RemoveEventListener) { }

  NewEventListener(edit: NewEventListener, handler: (event: Event) => void) {
    const event_name = edit.event_name;
    const mounted_node_id = edit.root;
    const scope = edit.scope;
    console.log('new event listener', event_name, mounted_node_id, scope);

    const element = this.nodes[edit.root];
    element.setAttribute(
      `dioxus-event-${event_name}`,
      `${scope}.${mounted_node_id}`
    );

    if (!this.listeners[event_name]) {
      this.listeners[event_name] = handler;
      this.root.addEventListener(event_name, handler);
    }
  }

  SetText(edit: SetText) {
    this.nodes[edit.root].textContent = edit.text;
  }

  SetAttribute(edit: SetAttribute) {
    // console.log("setting attr", edit);
    const name = edit.field;
    const value = edit.value;
    const ns = edit.ns;
    const node = this.nodes[edit.root];

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
  RemoveAttribute(edit: RemoveAttribute) {
    const name = edit.name;

    const node = this.nodes[edit.root];
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
    console.log("handling edits ", edits);
    this.stack.push(this.root);

    for (let edit of edits) {
      switch (edit.type) {
        case "AppendChildren":
          this.AppendChildren(edit);
          break;
        case "ReplaceWith":
          this.ReplaceWith(edit);
          break;
        case "InsertAfter":
          this.InsertAfter(edit);
          break;
        case "InsertBefore":
          this.InsertBefore(edit);
          break;
        case "Remove":
          this.Remove(edit);
          break;
        case "CreateTextNode":
          this.CreateTextNode(edit);
          break;
        case "CreateElement":
          this.CreateElement(edit);
          break;
        case "CreateElementNs":
          this.CreateElementNs(edit);
          break;
        case "CreatePlaceholder":
          this.CreatePlaceholder(edit);
          break;
        case "RemoveEventListener":
          this.RemoveEventListener(edit);
          break;
        case "NewEventListener":
          // todo: only on desktop should we make our own handler
          let handler = (event: Event) => {
            const target = event.target as Element | null;
            console.log("event", event);
            if (target != null) {

              const real_id = target.getAttribute(`dioxus-id`);

              const should_prevent_default = target.getAttribute(
                `dioxus-prevent-default`
              );

              let contents = serialize_event(event);

              if (should_prevent_default === `on${event.type}`) {
                event.preventDefault();
              }

              if (real_id == null) {
                return;
              }

              window.rpc.call("user_event", {
                event: (edit as NewEventListener).event_name,
                mounted_dom_id: parseInt(real_id),
                contents: contents,
              });
            }
          };
          this.NewEventListener(edit, handler);
          break;
        case "SetText":
          this.SetText(edit);
          break;
        case "SetAttribute":
          this.SetAttribute(edit);
          break;
        case "RemoveAttribute":
          this.RemoveAttribute(edit);
          break;
      }
    }
  }
}

function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.rpc.call("initialize");
  }
}


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


export { };
declare global {
  interface Window {
    interpreter: Interpreter;
    rpc: { call: (method: string, args?: any) => void };
  }
}


type Edits = DomEdit[];

main();
