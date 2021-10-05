
class Interpreter {
  constructor(root) {
    this.root = root;
    this.stack = [root];
    this.listeners = {
      "onclick": {}
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

  PopRoot(_edit) {
    this.stack.pop();
  }

  AppendChildren(edit) {
    let root = this.stack[this.stack.length - (1 + edit.many)];

    let to_add = this.stack.splice(this.stack.length - edit.many);

    for (let i = 0; i < edit.many; i++) {
      root.appendChild(to_add[i]);
    }
  }

  ReplaceWith(edit) {
    console.log(edit);
    let root = this.nodes[edit.root];
    let els = this.stack.splice(this.stack.length - edit.m);

    console.log(root);
    console.log(els);


    root.replaceWith(...els);
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
    this.stack.push(el);
  }

  CreateElementNs(edit) {
    let el = document.createElementNS(edit.ns, edit.tag);
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  CreatePlaceholder(edit) {
    let el = document.createElement("pre");
    // let el = document.createComment("vroot");
    this.stack.push(el);
    this.nodes[edit.root] = el;
  }

  RemoveEventListener(edit) { }

  SetText(edit) {
    this.top().textContent = edit.text;
  }

  SetAttribute(edit) {
    const name = edit.field;
    const value = edit.value;
    const ns = edit.ns;
    const node = this.top(this.stack);
    if (ns == "style") {
      node.style[name] = value;
    } else if (ns !== undefined) {
      node.setAttributeNS(ns, name, value);
    } else {
      node.setAttribute(name, value);
    }
    if (name === "value") {
      node.value = value;
    }
    if (name === "checked") {
      node.checked = true;
    }
    if (name === "selected") {
      node.selected = true;
    }
  }
  RemoveAttribute(edit) {
    const name = edit.field;
    const node = this.top(this.stack);
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

  InsertAfter(edit) {
    let old = this.nodes[edit.root];
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    // console.log("inserting nodes after", new_nodes, old);
    old.after(...new_nodes);
  }

  InsertBefore(edit) {
    let old = this.nodes[edit.root];
    let new_nodes = this.stack.splice(this.stack.length - edit.n);
    old.before(...new_nodes);
  }

  NewEventListener(edit) {
    const event_name = edit.event_name;
    const mounted_node_id = edit.root;
    const scope = edit.scope;

    const element = this.top();
    element.setAttribute(`dioxus-event-${event_name}`, `${scope}.${mounted_node_id}`);

    if (this.listeners[event_name] === undefined) {
      this.listeners[event_name] = "bla";

      this.root.addEventListener(event_name, (event) => {
        console.log("CLICKED");
        const target = event.target;
        const val = target.getAttribute(`dioxus-event-${event_name}`);
        if (val == null) {
          return;
        }

        const fields = val.split(".");
        const scope_id = parseInt(fields[0]);
        const real_id = parseInt(fields[1]);

        // console.log(`parsed event with scope_id ${scope_id} and real_id ${real_id}`);

        console.log("message fired");
        let contents = serialize_event(event);
        rpc.call('user_event', {
          event: event_name,
          scope: scope_id,
          mounted_dom_id: real_id,
          contents: contents,
        }).then((reply) => {
          console.log("reply received");

          // console.log(reply);
          this.stack.push(this.root);

          let edits = reply.edits;

          for (let x = 0; x < edits.length; x++) {
            let edit = edits[x];
            let f = this[edit.type];
            f.call(this, edit);
          }

          // console.log("initiated");
        })
      });
    }
  }
}

async function initialize() {
  let root = window.document.getElementById("_dioxusroot");
  const interpreter = new Interpreter(root);

  const reply = await rpc.call('initiate');

  let pre_rendered = reply.pre_rendered;
  if (pre_rendered !== undefined) {
    root.innerHTML = pre_rendered;
  }

  const edits = reply.edits;

  apply_edits(edits, interpreter);
}

function apply_edits(edits, interpreter) {
  console.log(edits);
  for (let x = 0; x < edits.length; x++) {
    let edit = edits[x];
    let f = interpreter[edit.type];
    f.call(interpreter, edit);
  }

  // console.log("stack completed: ", interpreter.stack);
}

function serialize_event(event) {
  let serializer = SerializeMap[event.type];
  if (serializer === undefined) {
    return {};
  } else {
    return serializer(event);
  }
}

const SerializeMap = {
  "copy": serialize_clipboard,
  "cut": serialize_clipboard,
  "paste": serialize_clipboard,

  "compositionend": serialize_composition,
  "compositionstart": serialize_composition,
  "compositionupdate": serialize_composition,

  "keydown": serialize_keyboard,
  "keypress": serialize_keyboard,
  "keyup": serialize_keyboard,

  "focus": serialize_focus,
  "blur": serialize_focus,

  "change": serialize_change,

  "input": serialize_form,
  "invalid": serialize_form,
  "reset": serialize_form,
  "submit": serialize_form,

  "click": serialize_mouse,
  "contextmenu": serialize_mouse,
  "doubleclick": serialize_mouse,
  "drag": serialize_mouse,
  "dragend": serialize_mouse,
  "dragenter": serialize_mouse,
  "dragexit": serialize_mouse,
  "dragleave": serialize_mouse,
  "dragover": serialize_mouse,
  "dragstart": serialize_mouse,
  "drop": serialize_mouse,
  "mousedown": serialize_mouse,
  "mouseenter": serialize_mouse,
  "mouseleave": serialize_mouse,
  "mousemove": serialize_mouse,
  "mouseout": serialize_mouse,
  "mouseover": serialize_mouse,
  "mouseup": serialize_mouse,

  "pointerdown": serialize_pointer,
  "pointermove": serialize_pointer,
  "pointerup": serialize_pointer,
  "pointercancel": serialize_pointer,
  "gotpointercapture": serialize_pointer,
  "lostpointercapture": serialize_pointer,
  "pointerenter": serialize_pointer,
  "pointerleave": serialize_pointer,
  "pointerover": serialize_pointer,
  "pointerout": serialize_pointer,

  "select": serialize_selection,

  "touchcancel": serialize_touch,
  "touchend": serialize_touch,
  "touchmove": serialize_touch,
  "touchstart": serialize_touch,

  "scroll": serialize_scroll,

  "wheel": serialize_wheel,

  "animationstart": serialize_animation,
  "animationend": serialize_animation,
  "animationiteration": serialize_animation,

  "transitionend": serialize_transition,

  "abort": serialize_media,
  "canplay": serialize_media,
  "canplaythrough": serialize_media,
  "durationchange": serialize_media,
  "emptied": serialize_media,
  "encrypted": serialize_media,
  "ended": serialize_media,
  "error": serialize_media,
  "loadeddata": serialize_media,
  "loadedmetadata": serialize_media,
  "loadstart": serialize_media,
  "pause": serialize_media,
  "play": serialize_media,
  "playing": serialize_media,
  "progress": serialize_media,
  "ratechange": serialize_media,
  "seeked": serialize_media,
  "seeking": serialize_media,
  "stalled": serialize_media,
  "suspend": serialize_media,
  "timeupdate": serialize_media,
  "volumechange": serialize_media,
  "waiting": serialize_media,

  "toggle": serialize_toggle
}

function serialize_clipboard(_event) {
  return {};
}
function serialize_composition(event) {
  return {
    data: event.data
  }
}
function serialize_keyboard(event) {
  return {
    alt_key: event.altKey,
    char_code: event.charCode,
    key: event.key,
    key_code: event.keyCode,
    ctrl_key: event.ctrlKey,
    locale: event.locale,
    location: event.location,
    meta_key: event.metaKey,
    repeat: event.repeat,
    shift_key: event.shiftKey,
    which: event.which,
  }
}
function serialize_focus(_event) {
  return {}
}
function serialize_change(_event) {
  return {}
}
function serialize_form(event) {
  let target = event.target;
  let value = target.value ?? target.textContent;
  return {
    value: value
  }
}
function serialize_mouse(event) {
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
  }
}

function serialize_pointer(event) {
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
  }
}

function serialize_selection(event) {
  return {}
}

function serialize_touch(event) {
  return {
    alt_key: event.altKey,
    ctrl_key: event.ctrlKey,
    meta_key: event.metaKey,
    shift_key: event.shiftKey,

    // changed_touches: event.changedTouches,
    // target_touches: event.targetTouches,
    // touches: event.touches,
  }
}
function serialize_scroll(event) {
  return {}
}

function serialize_wheel(event) {
  return {
    delta_x: event.deltaX,
    delta_y: event.deltaY,
    delta_z: event.deltaZ,
    delta_mode: event.deltaMode,
  }
}

function serialize_animation(event) {
  return {
    animation_name: event.animationName,
    elapsed_time: event.elapsedTime,
    pseudo_element: event.pseudoElement,
  }
}

function serialize_transition(event) {
  return {
    property_name: event.propertyName,
    elapsed_time: event.elapsedTime,
    pseudo_element: event.pseudoElement,
  }
}

function serialize_media(event) {
  return {}
}

function serialize_toggle(event) {
  return {}
}


initialize();
