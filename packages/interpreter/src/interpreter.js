class InterpreterConfig {
  constructor(intercept_link_redirects) {
    this.intercept_link_redirects = intercept_link_redirects;
  }
}

// this handler is only provided on the desktop and liveview implementations since this
// method is not used by the web implementation
async function handler(event, name, bubbles, config) {
  let target = event.target;
  if (target != null) {
    let preventDefaultRequests = null;
    // Some events can be triggered on text nodes, which don't have attributes
    if (target instanceof Element) {
      preventDefaultRequests = target.getAttribute(`dioxus-prevent-default`);
    }

    if (event.type === "click") {
      // todo call prevent default if it's the right type of event
      if (config.intercept_link_redirects) {
        let a_element = target.closest("a");
        if (a_element != null) {
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
      }

      // also prevent buttons from submitting
      if (target.tagName === "BUTTON" && event.type == "submit") {
        event.preventDefault();
      }
    }

    const realId = find_real_id(target);

    if (
      preventDefaultRequests &&
      preventDefaultRequests.includes(`on${event.type}`)
    ) {
      event.preventDefault();
    }

    if (event.type === "submit") {
      event.preventDefault();
    }

    let contents = await serialize_event(event);

    // TODO: this should be liveview only
    if (
      target.tagName === "INPUT" &&
      (event.type === "change" || event.type === "input")
    ) {
      const type = target.getAttribute("type");
      if (type === "file") {
        async function read_files() {
          const files = target.files;
          const file_contents = {};

          for (let i = 0; i < files.length; i++) {
            const file = files[i];

            file_contents[file.name] = Array.from(
              new Uint8Array(await file.arrayBuffer())
            );
          }
          let file_engine = {
            files: file_contents,
          };
          contents.files = file_engine;

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
        read_files();
        return;
      }
    }

    if (
      target.tagName === "FORM" &&
      (event.type === "submit" || event.type === "input")
    ) {
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

    if (
      target.tagName === "SELECT" &&
      event.type === "input"
    ) {
      const selectData = target.options;
      contents.values["options"] = [];
      for (let i = 0; i < selectData.length; i++) {
        let option = selectData[i];
        if (option.selected) {
          contents.values["options"].push(option.value.toString());
        }
      }
    }

    if (realId === null) {
      return;
    }
    window.ipc.postMessage(
      window.interpreter.serializeIpcMessage("user_event", {
        name: name,
        element: parseInt(realId),
        data: contents,
        bubbles,
      })
    );
  }
}

function find_real_id(target) {
  let realId = null;
  if (target instanceof Element) {
    realId = target.getAttribute(`data-dioxus-id`);
  }
  // walk the tree to find the real element
  while (realId == null) {
    // we've reached the root we don't want to send an event
    if (target.parentElement === null) {
      return;
    }

    target = target.parentElement;
    if (target instanceof Element) {
      realId = target.getAttribute(`data-dioxus-id`);
    }
  }
  return realId;
}

class ListenerMap {
  constructor(root) {
    // bubbling events can listen at the root element
    this.global = {};
    // non bubbling events listen at the element the listener was created at
    this.local = {};
    this.root = null;
  }

  create(event_name, element, bubbles, handler) {
    if (bubbles) {
      if (this.global[event_name] === undefined) {
        this.global[event_name] = {};
        this.global[event_name].active = 1;
        this.root.addEventListener(event_name, handler);
      } else {
        this.global[event_name].active++;
      }
    }
    else {
      const id = element.getAttribute("data-dioxus-id");
      if (!this.local[id]) {
        this.local[id] = {};
      }
      element.addEventListener(event_name, handler);
    }
  }

  remove(element, event_name, bubbles) {
    if (bubbles) {
      this.global[event_name].active--;
      if (this.global[event_name].active === 0) {
        this.root.removeEventListener(event_name, this.global[event_name].callback);
        delete this.global[event_name];
      }
    }
    else {
      const id = element.getAttribute("data-dioxus-id");
      delete this.local[id][event_name];
      if (this.local[id].length === 0) {
        delete this.local[id];
      }
      element.removeEventListener(event_name, this.global[event_name].callback);
    }
  }

  removeAllNonBubbling(element) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }
}
function LoadChild(array) {
  // iterate through each number and get that child
  node = stack[stack.length - 1];

  for (let i = 0; i < array.length; i++) {
    end = array[i];
    for (node = node.firstChild; end > 0; end--) {
      node = node.nextSibling;
    }
  }
  return node;
}
const listeners = new ListenerMap();
let nodes = [];
let stack = [];
let root;
const templates = {};
let node, els, end, k;

function AppendChildren(id, many) {
  root = nodes[id];
  els = stack.splice(stack.length - many);
  for (k = 0; k < many; k++) {
    root.appendChild(els[k]);
  }
}

window.interpreter = {}

window.interpreter.initialize = function (root) {
  nodes = [root];
  stack = [root];
  listeners.root = root;
}

window.interpreter.getClientRect = function (id) {
  const node = nodes[id];
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

window.interpreter.scrollTo = function (id, behavior) {
  const node = nodes[id];
  if (!node) {
    return false;
  }
  node.scrollIntoView({
    behavior: behavior,
  });
  return true;
}

/// Set the focus on the element
window.interpreter.setFocus = function (id, focus) {
  const node = nodes[id];
  if (!node) {
    return false;
  }
  if (focus) {
    node.focus();
  } else {
    node.blur();
  }
  return true;
}

function get_mouse_data(event) {
  const {
    altKey,
    button,
    buttons,
    clientX,
    clientY,
    ctrlKey,
    metaKey,
    offsetX,
    offsetY,
    pageX,
    pageY,
    screenX,
    screenY,
    shiftKey,
  } = event;
  return {
    alt_key: altKey,
    button: button,
    buttons: buttons,
    client_x: clientX,
    client_y: clientY,
    ctrl_key: ctrlKey,
    meta_key: metaKey,
    offset_x: offsetX,
    offset_y: offsetY,
    page_x: pageX,
    page_y: pageY,
    screen_x: screenX,
    screen_y: screenY,
    shift_key: shiftKey,
  };
}

async function serialize_event(event) {
  switch (event.type) {
    case "copy":
    case "cut":
    case "past": {
      return {};
    }
    case "compositionend":
    case "compositionstart":
    case "compositionupdate": {
      let { data } = event;
      return {
        data,
      };
    }
    case "keydown":
    case "keypress":
    case "keyup": {
      let {
        charCode,
        isComposing,
        key,
        altKey,
        ctrlKey,
        metaKey,
        keyCode,
        shiftKey,
        location,
        repeat,
        which,
        code,
      } = event;
      return {
        char_code: charCode,
        is_composing: isComposing,
        key: key,
        alt_key: altKey,
        ctrl_key: ctrlKey,
        meta_key: metaKey,
        key_code: keyCode,
        shift_key: shiftKey,
        location: location,
        repeat: repeat,
        which: which,
        code,
      };
    }
    case "focus":
    case "blur": {
      return {};
    }
    case "change": {
      let target = event.target;
      let value;
      if (target.type === "checkbox" || target.type === "radio") {
        value = target.checked ? "true" : "false";
      } else {
        value = target.value ?? target.textContent;
      }
      return {
        value: value,
        values: {},
      };
    }
    case "input":
    case "invalid":
    case "reset":
    case "submit": {
      let target = event.target;
      let value = target.value ?? target.textContent;

      if (target.type === "checkbox") {
        value = target.checked ? "true" : "false";
      }

      return {
        value: value,
        values: {},
      };
    }
    case "drag":
    case "dragend":
    case "dragenter":
    case "dragexit":
    case "dragleave":
    case "dragover":
    case "dragstart":
    case "drop": {
      let files = null;
      if (event.dataTransfer && event.dataTransfer.files) {
        files = await serializeFileList(event.dataTransfer.files);
      }

      return { mouse: get_mouse_data(event), files };
    }
    case "click":
    case "contextmenu":
    case "doubleclick":
    case "dblclick":
    case "mousedown":
    case "mouseenter":
    case "mouseleave":
    case "mousemove":
    case "mouseout":
    case "mouseover":
    case "mouseup": {
      return get_mouse_data(event);
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
      } = event;
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
      const { altKey, ctrlKey, metaKey, shiftKey } = event;
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
      const { deltaX, deltaY, deltaZ, deltaMode } = event;
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
      const { animationName, elapsedTime, pseudoElement } = event;
      return {
        animation_name: animationName,
        elapsed_time: elapsedTime,
        pseudo_element: pseudoElement,
      };
    }
    case "transitionend": {
      const { propertyName, elapsedTime, pseudoElement } = event;
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
window.interpreter.serializeIpcMessage = function (method, params = {}) {
  return JSON.stringify({ method, params });
}

function is_element_node(node) {
  return node.nodeType == 1;
}

function event_bubbles(event) {
  switch (event) {
    case "copy":
      return true;
    case "cut":
      return true;
    case "paste":
      return true;
    case "compositionend":
      return true;
    case "compositionstart":
      return true;
    case "compositionupdate":
      return true;
    case "keydown":
      return true;
    case "keypress":
      return true;
    case "keyup":
      return true;
    case "focus":
      return false;
    case "focusout":
      return true;
    case "focusin":
      return true;
    case "blur":
      return false;
    case "change":
      return true;
    case "input":
      return true;
    case "invalid":
      return true;
    case "reset":
      return true;
    case "submit":
      return true;
    case "click":
      return true;
    case "contextmenu":
      return true;
    case "doubleclick":
      return true;
    case "dblclick":
      return true;
    case "drag":
      return true;
    case "dragend":
      return true;
    case "dragenter":
      return false;
    case "dragexit":
      return false;
    case "dragleave":
      return true;
    case "dragover":
      return true;
    case "dragstart":
      return true;
    case "drop":
      return true;
    case "mousedown":
      return true;
    case "mouseenter":
      return false;
    case "mouseleave":
      return false;
    case "mousemove":
      return true;
    case "mouseout":
      return true;
    case "scroll":
      return false;
    case "mouseover":
      return true;
    case "mouseup":
      return true;
    case "pointerdown":
      return true;
    case "pointermove":
      return true;
    case "pointerup":
      return true;
    case "pointercancel":
      return true;
    case "gotpointercapture":
      return true;
    case "lostpointercapture":
      return true;
    case "pointerenter":
      return false;
    case "pointerleave":
      return false;
    case "pointerover":
      return true;
    case "pointerout":
      return true;
    case "select":
      return true;
    case "touchcancel":
      return true;
    case "touchend":
      return true;
    case "touchmove":
      return true;
    case "touchstart":
      return true;
    case "wheel":
      return true;
    case "abort":
      return false;
    case "canplay":
      return false;
    case "canplaythrough":
      return false;
    case "durationchange":
      return false;
    case "emptied":
      return false;
    case "encrypted":
      return true;
    case "ended":
      return false;
    case "error":
      return false;
    case "loadeddata":
    case "loadedmetadata":
    case "loadstart":
    case "load":
      return false;
    case "pause":
      return false;
    case "play":
      return false;
    case "playing":
      return false;
    case "progress":
      return false;
    case "ratechange":
      return false;
    case "seeked":
      return false;
    case "seeking":
      return false;
    case "stalled":
      return false;
    case "suspend":
      return false;
    case "timeupdate":
      return false;
    case "volumechange":
      return false;
    case "waiting":
      return false;
    case "animationstart":
      return true;
    case "animationend":
      return true;
    case "animationiteration":
      return true;
    case "transitionend":
      return true;
    case "toggle":
      return true;
    case "mounted":
      return false;
  }

  return true;
}
