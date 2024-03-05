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
var getTargetId = function(target) {
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
var JSChannel_;
if (RawInterpreter) {
  JSChannel_ = RawInterpreter;
}

class NativeInterpreter extends JSChannel_ {
  intercept_link_redirects;
  ipc;
  editsPath;
  liveview;
  constructor(editsPath) {
    super();
    this.editsPath = editsPath;
  }
  initialize(root) {
    this.intercept_link_redirects = true;
    this.liveview = false;
    const dragEventHandler = (e) => {
    };
    window.addEventListener("dragover", function(e) {
      if (e.target instanceof Element && e.target.tagName != "INPUT") {
        e.preventDefault();
      }
    }, false);
    window.addEventListener("drop", function(e) {
      let target = e.target;
      if (!(target instanceof Element)) {
        return;
      }
      e.preventDefault();
    }, false);
    window.addEventListener("click", (event) => {
      const target = event.target;
      if (target instanceof HTMLInputElement && target.getAttribute("type") === "file") {
        let target_id = getTargetId(target);
        if (target_id !== null) {
          const message = this.serializeIpcMessage("file_dialog", {
            event: "change&input",
            accept: target.getAttribute("accept"),
            directory: target.getAttribute("webkitdirectory") === "true",
            multiple: target.hasAttribute("multiple"),
            target: target_id,
            bubbles: event.bubbles
          });
          this.ipc.postMessage(message);
        }
        event.preventDefault();
      }
    });
    this.ipc = window.ipc;
    const handler = (event) => this.handleEvent(event, event.type, true);
    super.initialize(root, handler);
  }
  serializeIpcMessage(method, params = {}) {
    return JSON.stringify({ method, params });
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
  loadChild(array) {
    let node = this.stack[this.stack.length - 1];
    for (let i = 0;i < array.length; i++) {
      let end = array[i];
      for (node = node.firstChild;end > 0; end--) {
        node = node.nextSibling;
      }
    }
    return node;
  }
  appendChildren(id, many) {
    const root = this.nodes[id];
    const els = this.stack.splice(this.stack.length - many);
    for (let k = 0;k < many; k++) {
      root.appendChild(els[k]);
    }
  }
  handleEvent(event, name, bubbles) {
    const target = event.target;
    const realId = getTargetId(target);
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
  waitForRequest(headless) {
    fetch(new Request(this.editsPath)).then((response) => response.arrayBuffer()).then((bytes) => {
      if (headless) {
        this.run_from_bytes(bytes);
      } else {
        requestAnimationFrame(() => this.run_from_bytes(bytes));
      }
      this.waitForRequest(headless);
    });
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
}
export {
  NativeInterpreter
};
