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

function serialize_event(event) {
  switch (event.type) {
    case "copy":
    case "cut":
    case "past":
      return {};

    case "compositionend":
    case "compositionstart":
    case "compositionupdate":
      return {
        data: event.data,
      };

    case "keydown":
    case "keypress":
    case "keyup":
      return {
        char_code: event.charCode,
        key: event.key,
        alt_key: event.altKey,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        key_code: event.keyCode,
        shift_key: event.shiftKey,
        locale: "locale",
        location: event.location,
        repeat: event.repeat,
        which: event.which,
        // locale: event.locale,
      };

    case "focus":
    case "blur":
      return {};

    case "change":
      let target = event.target;
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
      let target = event.target;
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
    case "mouseup":
      return {
        alt_key: event.altKey,
        button: event.button,
        buttons: event.buttons,
        client_x: event.clientX,
        client_y: event.clientY,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        page_x: event.pageX,
        page_y: event.pageY,
        screen_x: event.screenX,
        screen_y: event.screenY,
        shift_key: event.shiftKey,
      };

    case "pointerdown":
    case "pointermove":
    case "pointerup":
    case "pointercancel":
    case "gotpointercapture":
    case "lostpointercapture":
    case "pointerenter":
    case "pointerleave":
    case "pointerover":
    case "pointerout":
      return {
        alt_key: event.altKey,
        button: event.button,
        buttons: event.buttons,
        client_x: event.clientX,
        client_y: event.clientY,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        page_x: event.pageX,
        page_y: event.pageY,
        screen_x: event.screenX,
        screen_y: event.screenY,
        shift_key: event.shiftKey,
        pointer_id: event.pointerId,
        width: event.width,
        height: event.height,
        pressure: event.pressure,
        tangential_pressure: event.tangentialPressure,
        tilt_x: event.tiltX,
        tilt_y: event.tiltY,
        twist: event.twist,
        pointer_type: event.pointerType,
        is_primary: event.isPrimary,
      };

    case "select":
      return {};

    case "touchcancel":
    case "touchend":
    case "touchmove":
    case "touchstart":
      return {
        alt_key: event.altKey,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        shift_key: event.shiftKey,

        // changed_touches: event.changedTouches,
        // target_touches: event.targetTouches,
        // touches: event.touches,
      };

    case "scroll":
      return {};

    case "wheel":
      return {
        delta_x: event.deltaX,
        delta_y: event.deltaY,
        delta_z: event.deltaZ,
        delta_mode: event.deltaMode,
      };

    case "animationstart":
    case "animationend":
    case "animationiteration":
      return {
        animation_name: event.animationName,
        elapsed_time: event.elapsedTime,
        pseudo_element: event.pseudoElement,
      };

    case "transitionend":
      return {
        property_name: event.propertyName,
        elapsed_time: event.elapsedTime,
        pseudo_element: event.pseudoElement,
      };

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

class Interpreter {
  constructor(root) {
    this.root = root;
    this.stack = [root];
    this.listeners = {
      onclick: {},
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

  PushRoot(edit) {
    const id = edit.root;
    const node = this.nodes[id];
    this.stack.push(node);
  }

  AppendChildren(edit) {
    let root = this.stack[this.stack.length - (1 + edit.many)];

    let to_add = this.stack.splice(this.stack.length - edit.many);

    for (let i = 0; i < edit.many; i++) {
      root.appendChild(to_add[i]);
    }
  }

  ReplaceWith(edit) {
    let root = this.nodes[edit.root];
    let els = this.stack.splice(this.stack.length - edit.m);

    root.replaceWith(...els);
  }

  InsertAfter(edit) {
    let old = this.nodes[edit.root];
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    old.after(...new_nodes);
  }

  InsertBefore(edit) {
    let old = this.nodes[edit.root];
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    old.before(...new_nodes);
  }

  Remove(edit) {
    let node = this.nodes[edit.root];
    if (node !== undefined) {
      node.remove();
    }
  }

  CreateTextNode(edit) {
    const node = document.createTextNode(edit.text);
    this.nodes[edit.root] = node;
    this.stack.push(node);
  }

  CreateElement(edit) {
    const tagName = edit.tag;
    const el = document.createElement(tagName);
    this.nodes[edit.root] = el;
    el.setAttribute("dioxus-id", edit.root);
    this.stack.push(el);
  }

  CreateElementNs(edit) {
    let el = document.createElementNS(edit.ns, edit.tag);
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  CreatePlaceholder(edit) {
    let el = document.createElement("pre");
    el.hidden = true;
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  RemoveEventListener(edit) {}

  NewEventListener(edit) {
    const event_name = edit.event_name;
    const mounted_node_id = edit.root;
    const scope = edit.scope;

    const element = this.nodes[edit.root];
    element.setAttribute(
      `dioxus-event-${event_name}`,
      `${scope}.${mounted_node_id}`
    );

    if (this.listeners[event_name] === undefined) {
      this.listeners[event_name] = true;

      this.root.addEventListener(event_name, (event) => {

        const target = event.target;
        const real_id = target.getAttribute(`dioxus-id`);

        const should_prevent_default = target.getAttribute(
          `dioxus-prevent-default`
        );

        let contents = serialize_event(event);

        if (should_prevent_default === `on${event.type}`) {
          event.preventDefault();
        }

        if (event.type == "submit") {
          event.preventDefault();
        }
        
        if (event.type == "click") {
          event.preventDefault();
          if (should_prevent_default !== `onclick`) {
            console.log(event.target.getAttribute("href"));
            if(element.tagName == "A") {
              rpc.call("browser_open", {
                mounted_dom_id: parseInt(real_id),
                href: event.target.getAttribute("href")
              })
            }
          }
        }

        if (real_id == null) {
          return;
        }

        rpc.call("user_event", {
          event: event_name,
          mounted_dom_id: parseInt(real_id),
          contents: contents,
        });
      });
    }
  }

  SetText(edit) {
    this.nodes[edit.root].textContent = edit.text;
  }

  SetAttribute(edit) {

    // console.log("setting attr", edit);
    
    const name = edit.field;
    const value = edit.value;
    const ns = edit.ns;
    const node = this.nodes[edit.root];

    if (ns == "style") {
      node.style[name] = value;
    } else if (ns != null || ns != undefined) {
      node.setAttributeNS(ns, name, value);
    } else {
      switch (name) {
        case "value":
          if (value != node.value) {
            node.value = value;
          }
          break;
        case "checked":
          node.checked = value === "true";
          break;
        case "selected":
          node.selected = value === "true";
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
  RemoveAttribute(edit) {
    const name = edit.field;
    const node = this.nodes[edit.root];
    node.removeAttribute(name);

    if (name === "value") {
      node.value = null;
    }
    if (name === "checked") {
      node.checked = false;
    }
    if (name === "selected") {
      node.selected = false;
    }
  }

  handleEdits(edits) {
    this.stack.push(this.root);

    for (let x = 0; x < edits.length; x++) {
      let edit = edits[x];
      let f = this[edit.type];
      f.call(this, edit);
    }
  }
}

function main() {
  let root = window.document.getElementById("main");
  window.interpreter = new Interpreter(root);
  rpc.call("initialize");
}

main();
