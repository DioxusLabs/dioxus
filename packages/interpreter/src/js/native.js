// src/ts/set_attribute.ts
function setAttributeInner(node, field, value, ns) {
  if (ns === "style") {
    node.style.setProperty(field, value);
    return;
  }
  if (!!ns) {
    node.setAttributeNS(ns, field, value);
    return;
  }
  switch (field) {
    case "value":
      if (node.value !== value) {
        node.value = value;
      }
      break;
    case "initial_value":
      node.defaultValue = value;
      break;
    case "checked":
      node.checked = truthy(value);
      break;
    case "initial_checked":
      node.defaultChecked = truthy(value);
      break;
    case "selected":
      node.selected = truthy(value);
      break;
    case "initial_selected":
      node.defaultSelected = truthy(value);
      break;
    case "dangerous_inner_html":
      node.innerHTML = value;
      break;
    default:
      if (!truthy(value) && isBoolAttr(field)) {
        node.removeAttribute(field);
      } else {
        node.setAttribute(field, value);
      }
  }
}
var truthy = function(val) {
  return val === "true" || val === true;
};
var isBoolAttr = function(field) {
  switch (field) {
    case "allowfullscreen":
    case "allowpaymentrequest":
    case "async":
    case "autofocus":
    case "autoplay":
    case "checked":
    case "controls":
    case "default":
    case "defer":
    case "disabled":
    case "formnovalidate":
    case "hidden":
    case "ismap":
    case "itemscope":
    case "loop":
    case "multiple":
    case "muted":
    case "nomodule":
    case "novalidate":
    case "open":
    case "playsinline":
    case "readonly":
    case "required":
    case "reversed":
    case "selected":
    case "truespeed":
    case "webkitdirectory":
      return true;
    default:
      return false;
  }
};

// src/ts/core.ts
class BaseInterpreter {
  global;
  local;
  root;
  handler;
  nodes;
  stack;
  templates;
  m;
  constructor(root, handler) {
    this.handler = handler;
    this.initialize(root);
  }
  initialize(root, handler = null) {
    this.global = {};
    this.local = {};
    this.root = root;
    this.nodes = [root];
    this.stack = [root];
    this.templates = {};
    if (handler) {
      this.handler = handler;
    }
  }
  createListener(event_name, element, bubbles) {
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
  removeListener(element, event_name, bubbles) {
    if (bubbles) {
      this.removeBubblingListener(event_name);
    } else {
      this.removeNonBubblingListener(element, event_name);
    }
  }
  removeBubblingListener(event_name) {
    this.global[event_name].active--;
    if (this.global[event_name].active === 0) {
      this.root.removeEventListener(event_name, this.global[event_name].callback);
      delete this.global[event_name];
    }
  }
  removeNonBubblingListener(element, event_name) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id][event_name];
    if (Object.keys(this.local[id]).length === 0) {
      delete this.local[id];
    }
    element.removeEventListener(event_name, this.handler);
  }
  removeAllNonBubblingListeners(element) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }
  getNode(id) {
    return this.nodes[id];
  }
  appendChildren(id, many) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0;k < many; k++) {
      root.appendChild(els[k]);
    }
  }
  loadChild(ptr, len) {
    let node = this.stack[this.stack.length - 1];
    let ptr_end = ptr + len;
    for (;ptr < ptr_end; ptr++) {
      let end = this.m.getUint8(ptr);
      for (node = node.firstChild;end > 0; end--) {
        node = node.nextSibling;
      }
    }
    return node;
  }
  saveTemplate(nodes, tmpl_id) {
    this.templates[tmpl_id] = nodes;
  }
  hydrateRoot(ids) {
    const hydrateNodes = document.querySelectorAll("[data-node-hydration]");
    for (let i = 0;i < hydrateNodes.length; i++) {
      const hydrateNode = hydrateNodes[i];
      const hydration = hydrateNode.getAttribute("data-node-hydration");
      const split = hydration.split(",");
      const id = ids[parseInt(split[0])];
      this.nodes[id] = hydrateNode;
      if (split.length > 1) {
        hydrateNode.listening = split.length - 1;
        hydrateNode.setAttribute("data-dioxus-id", id.toString());
        for (let j = 1;j < split.length; j++) {
          const listener = split[j];
          const split2 = listener.split(":");
          const event_name = split2[0];
          const bubbles = split2[1] === "1";
          this.createListener(event_name, hydrateNode, bubbles);
        }
      }
    }
    const treeWalker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT);
    let currentNode = treeWalker.nextNode();
    while (currentNode) {
      const id = currentNode.textContent;
      const split = id.split("node-id");
      if (split.length > 1) {
        this.nodes[ids[parseInt(split[1])]] = currentNode.nextSibling;
      }
      currentNode = treeWalker.nextNode();
    }
  }
  setAttributeInner(node, field, value, ns) {
    setAttributeInner(node, field, value, ns);
  }
}

// src/ts/form.ts
function retriveValues(event, target) {
  let contents = {
    values: {}
  };
  let form = target.closest("form");
  if (form) {
    if (event.type === "input" || event.type === "change" || event.type === "submit" || event.type === "reset" || event.type === "click") {
      contents = retrieveFormValues(form);
    }
  }
  return contents;
}
function retrieveFormValues(form) {
  const formData = new FormData(form);
  const contents = {};
  formData.forEach((value, key) => {
    if (contents[key]) {
      contents[key] += "," + value;
    } else {
      contents[key] = value;
    }
  });
  return {
    valid: form.checkValidity(),
    values: contents
  };
}
function retriveSelectValue(target) {
  let options = target.selectedOptions;
  let values = [];
  for (let i = 0;i < options.length; i++) {
    values.push(options[i].value);
  }
  return values;
}

// src/ts/serialize.ts
function serializeEvent(event, target) {
  let contents = {};
  let extend = (obj) => contents = { ...contents, ...obj };
  if (event instanceof WheelEvent) {
    extend(serializeWheelEvent(event));
  }
  if (event instanceof MouseEvent) {
    extend(serializeMouseEvent(event));
  }
  if (event instanceof KeyboardEvent) {
    extend(serializeKeyboardEvent(event));
  }
  if (event instanceof InputEvent) {
    extend(serializeInputEvent(event, target));
  }
  if (event instanceof PointerEvent) {
    extend(serializePointerEvent(event));
  }
  if (event instanceof AnimationEvent) {
    extend(serializeAnimationEvent(event));
  }
  if (event instanceof TransitionEvent) {
    extend({ property_name: event.propertyName, elapsed_time: event.elapsedTime, pseudo_element: event.pseudoElement });
  }
  if (event instanceof CompositionEvent) {
    extend({ data: event.data });
  }
  if (event instanceof DragEvent) {
    extend(serializeDragEvent(event));
  }
  if (event instanceof FocusEvent) {
    extend({});
  }
  if (event instanceof ClipboardEvent) {
    extend({});
  }
  if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent) {
    extend(serializeTouchEvent(event));
  }
  if (event.type === "submit" || event.type === "reset" || event.type === "click" || event.type === "change" || event.type === "input") {
    extend(serializeInputEvent(event, target));
  }
  if (event instanceof DragEvent) {
  }
  return contents;
}
var serializeInputEvent = function(event, target) {
  let contents = {};
  if (target instanceof HTMLElement) {
    let values = retriveValues(event, target);
    contents.values = values.values;
    contents.valid = values.valid;
  }
  if (event.target instanceof HTMLInputElement) {
    let target2 = event.target;
    let value = target2.value ?? target2.textContent ?? "";
    if (target2.type === "checkbox") {
      value = target2.checked ? "true" : "false";
    } else if (target2.type === "radio") {
      value = target2.value;
    }
    contents.value = value;
  }
  if (event.target instanceof HTMLTextAreaElement) {
    contents.value = event.target.value;
  }
  if (event.target instanceof HTMLSelectElement) {
    contents.value = retriveSelectValue(event.target).join(",");
  }
  if (contents.value === undefined) {
    contents.value = "";
  }
  return contents;
};
var serializeWheelEvent = function(event) {
  return {
    delta_x: event.deltaX,
    delta_y: event.deltaY,
    delta_z: event.deltaZ,
    delta_mode: event.deltaMode
  };
};
var serializeTouchEvent = function(event) {
  return {
    alt_key: event.altKey,
    ctrl_key: event.ctrlKey,
    meta_key: event.metaKey,
    shift_key: event.shiftKey,
    changed_touches: event.changedTouches,
    target_touches: event.targetTouches,
    touches: event.touches
  };
};
var serializePointerEvent = function(event) {
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
    is_primary: event.isPrimary
  };
};
var serializeMouseEvent = function(event) {
  return {
    alt_key: event.altKey,
    button: event.button,
    buttons: event.buttons,
    client_x: event.clientX,
    client_y: event.clientY,
    ctrl_key: event.ctrlKey,
    meta_key: event.metaKey,
    offset_x: event.offsetX,
    offset_y: event.offsetY,
    page_x: event.pageX,
    page_y: event.pageY,
    screen_x: event.screenX,
    screen_y: event.screenY,
    shift_key: event.shiftKey
  };
};
var serializeKeyboardEvent = function(event) {
  return {
    char_code: event.charCode,
    is_composing: event.isComposing,
    key: event.key,
    alt_key: event.altKey,
    ctrl_key: event.ctrlKey,
    meta_key: event.metaKey,
    key_code: event.keyCode,
    shift_key: event.shiftKey,
    location: event.location,
    repeat: event.repeat,
    which: event.which,
    code: event.code
  };
};
var serializeAnimationEvent = function(event) {
  return {
    animation_name: event.animationName,
    elapsed_time: event.elapsedTime,
    pseudo_element: event.pseudoElement
  };
};
var serializeDragEvent = function(event) {
  return {
    mouse: {
      alt_key: event.altKey,
      ctrl_key: event.ctrlKey,
      meta_key: event.metaKey,
      shift_key: event.shiftKey,
      ...serializeMouseEvent(event)
    },
    files: {
      files: {
        a: [1, 2, 3]
      }
    }
  };
};

// src/ts/native.ts
var targetId = function(target) {
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
};

class PlatformInterpreter extends BaseInterpreter {
  intercept_link_redirects;
  ipc;
  liveview;
  constructor(root) {
    super(root, (event) => this.handleEvent(event, event.type, true));
    this.intercept_link_redirects = true;
    this.liveview = false;
    window.addEventListener("dragover", function(e) {
      if (e.target instanceof Element && e.target.tagName != "INPUT") {
        e.preventDefault();
      }
    }, false);
    window.addEventListener("drop", function(e) {
      if (e.target instanceof Element && e.target.tagName != "INPUT") {
        e.preventDefault();
      }
    }, false);
    this.ipc = window.ipc;
  }
  serializeIpcMessage(method, params = {}) {
    return JSON.stringify({ method, params });
  }
  setAttributeInner(node, field, value, ns) {
    setAttributeInner(node, field, value, ns);
  }
  scrollTo(id, behavior) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      node.scrollIntoView({ behavior });
    }
  }
  getClientRect(id) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      const rect = node.getBoundingClientRect();
      return {
        type: "GetClientRect",
        origin: [rect.x, rect.y],
        size: [rect.width, rect.height]
      };
    }
  }
  setFocus(id, focus) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) {
      if (focus) {
        node.focus();
      } else {
        node.blur();
      }
    }
  }
  LoadChild(array) {
    let node = this.stack[this.stack.length - 1];
    for (let i = 0;i < array.length; i++) {
      let end = array[i];
      for (node = node.firstChild;end > 0; end--) {
        node = node.nextSibling;
      }
    }
    return node;
  }
  AppendChildren(id, many) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0;k < many; k++) {
      root.appendChild(els[k]);
    }
  }
  handleEvent(event, name, bubbles) {
    const target = event.target;
    const realId = targetId(target);
    const contents = serializeEvent(event, target);
    let body = {
      name,
      data: contents,
      element: realId,
      bubbles
    };
    this.preventDefaults(event, target);
    if (this.liveview) {
      if (target instanceof HTMLInputElement && (event.type === "change" || event.type === "input")) {
        if (target.getAttribute("type") === "file") {
          this.readFiles(target, contents, bubbles, realId, name);
        }
      }
    } else {
      const message = this.serializeIpcMessage("user_event", body);
      this.ipc.postMessage(message);
    }
  }
  async readFiles(target, contents, bubbles, realId, name) {
    let files = target.files;
    let file_contents = {};
    for (let i = 0;i < files.length; i++) {
      const file = files[i];
      file_contents[file.name] = Array.from(new Uint8Array(await file.arrayBuffer()));
    }
    contents.files = { files: file_contents };
    const message = this.serializeIpcMessage("user_event", {
      name,
      element: realId,
      data: contents,
      bubbles
    });
    this.ipc.postMessage(message);
  }
  preventDefaults(event, target) {
    let preventDefaultRequests = null;
    if (target instanceof Element) {
      preventDefaultRequests = target.getAttribute(`dioxus-prevent-default`);
    }
    if (preventDefaultRequests && preventDefaultRequests.includes(`on${event.type}`)) {
      event.preventDefault();
    }
    if (event.type === "submit") {
      event.preventDefault();
    }
    if (target instanceof Element && event.type === "click") {
      this.handleClickNavigate(event, target, preventDefaultRequests);
    }
  }
  handleClickNavigate(event, target, preventDefaultRequests) {
    if (!this.intercept_link_redirects) {
      return;
    }
    if (target.tagName === "BUTTON" && event.type == "submit") {
      event.preventDefault();
    }
    let a_element = target.closest("a");
    if (a_element == null) {
      return;
    }
    event.preventDefault();
    let elementShouldPreventDefault = preventDefaultRequests && preventDefaultRequests.includes(`onclick`);
    let aElementShouldPreventDefault = a_element.getAttribute(`dioxus-prevent-default`);
    let linkShouldPreventDefault = aElementShouldPreventDefault && aElementShouldPreventDefault.includes(`onclick`);
    if (!elementShouldPreventDefault && !linkShouldPreventDefault) {
      const href = a_element.getAttribute("href");
      if (href !== "" && href !== null && href !== undefined) {
        this.ipc.postMessage(this.serializeIpcMessage("browser_open", { href }));
      }
    }
  }
}
export {
  PlatformInterpreter
};
