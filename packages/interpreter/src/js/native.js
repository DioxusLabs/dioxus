function retrieveValues(event, target) {
  let contents = { values: {} },
    form = target.closest("form");
  if (form) {
    if (
      event.type === "input" ||
      event.type === "change" ||
      event.type === "submit" ||
      event.type === "reset" ||
      event.type === "click"
    )
      contents = retrieveFormValues(form);
  }
  return contents;
}
function retrieveFormValues(form) {
  const formData = new FormData(form),
    contents = {};
  return (
    formData.forEach((value, key) => {
      if (contents[key]) contents[key].push(value);
      else contents[key] = [value];
    }),
    { valid: form.checkValidity(), values: contents }
  );
}
function retrieveSelectValue(target) {
  let options = target.selectedOptions,
    values = [];
  for (let i = 0; i < options.length; i++) values.push(options[i].value);
  return values;
}
function serializeEvent(event, target) {
  let contents = {},
    extend = (obj) => (contents = { ...contents, ...obj });
  if (event instanceof WheelEvent) extend(serializeWheelEvent(event));
  if (event instanceof MouseEvent) extend(serializeMouseEvent(event));
  if (event instanceof KeyboardEvent) extend(serializeKeyboardEvent(event));
  if (event instanceof InputEvent) extend(serializeInputEvent(event, target));
  if (event instanceof PointerEvent) extend(serializePointerEvent(event));
  if (event instanceof AnimationEvent) extend(serializeAnimationEvent(event));
  if (event instanceof TransitionEvent)
    extend({
      property_name: event.propertyName,
      elapsed_time: event.elapsedTime,
      pseudo_element: event.pseudoElement,
    });
  if (event instanceof CompositionEvent) extend({ data: event.data });
  if (event instanceof DragEvent) extend(serializeDragEvent(event));
  if (event instanceof FocusEvent) extend({});
  if (event instanceof ClipboardEvent) extend({});
  if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent)
    extend(serializeTouchEvent(event));
  if (
    event.type === "submit" ||
    event.type === "reset" ||
    event.type === "click" ||
    event.type === "change" ||
    event.type === "input"
  )
    extend(serializeInputEvent(event, target));
  if (event instanceof DragEvent);
  return contents;
}
var serializeInputEvent = function (event, target) {
    let contents = {};
    if (target instanceof HTMLElement) {
      let values = retrieveValues(event, target);
      (contents.values = values.values), (contents.valid = values.valid);
    }
    if (event.target instanceof HTMLInputElement) {
      let target2 = event.target,
        value = target2.value ?? target2.textContent ?? "";
      if (target2.type === "checkbox")
        value = target2.checked ? "true" : "false";
      else if (target2.type === "radio") value = target2.value;
      contents.value = value;
    }
    if (event.target instanceof HTMLTextAreaElement)
      contents.value = event.target.value;
    if (event.target instanceof HTMLSelectElement)
      contents.value = retrieveSelectValue(event.target).join(",");
    if (contents.value === void 0) contents.value = "";
    return contents;
  },
  serializeWheelEvent = function (event) {
    return {
      delta_x: event.deltaX,
      delta_y: event.deltaY,
      delta_z: event.deltaZ,
      delta_mode: event.deltaMode,
    };
  },
  serializeTouchEvent = function (event) {
    return {
      alt_key: event.altKey,
      ctrl_key: event.ctrlKey,
      meta_key: event.metaKey,
      shift_key: event.shiftKey,
      changed_touches: event.changedTouches,
      target_touches: event.targetTouches,
      touches: event.touches,
    };
  },
  serializePointerEvent = function (event) {
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
  },
  serializeMouseEvent = function (event) {
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
      shift_key: event.shiftKey,
    };
  },
  serializeKeyboardEvent = function (event) {
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
      code: event.code,
    };
  },
  serializeAnimationEvent = function (event) {
    return {
      animation_name: event.animationName,
      elapsed_time: event.elapsedTime,
      pseudo_element: event.pseudoElement,
    };
  },
  serializeDragEvent = function (event) {
    let files = void 0;
    if (
      event.dataTransfer &&
      event.dataTransfer.files &&
      event.dataTransfer.files.length > 0
    )
      files = { files: { placeholder: [] } };
    return {
      mouse: {
        alt_key: event.altKey,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        shift_key: event.shiftKey,
        ...serializeMouseEvent(event),
      },
      files,
    };
  };
var getTargetId = function (target) {
    if (!(target instanceof Node)) return null;
    let ourTarget = target,
      realId = null;
    while (realId == null) {
      if (ourTarget === null) return null;
      if (ourTarget instanceof Element)
        realId = ourTarget.getAttribute("data-dioxus-id");
      ourTarget = ourTarget.parentNode;
    }
    return parseInt(realId);
  },
  JSChannel_;
if (RawInterpreter !== void 0 && RawInterpreter !== null)
  JSChannel_ = RawInterpreter;
class NativeInterpreter extends JSChannel_ {
  intercept_link_redirects;
  ipc;
  editsPath;
  kickStylesheets;
  queuedBytes = [];
  liveview;
  constructor(editsPath) {
    super();
    (this.editsPath = editsPath), (this.kickStylesheets = !1);
  }
  initialize(root) {
    (this.intercept_link_redirects = !0),
      (this.liveview = !1),
      window.addEventListener(
        "dragover",
        function (e) {
          if (e.target instanceof Element && e.target.tagName != "INPUT")
            e.preventDefault();
        },
        !1
      ),
      window.addEventListener(
        "drop",
        function (e) {
          if (!(e.target instanceof Element)) return;
          e.preventDefault();
        },
        !1
      ),
      window.addEventListener("click", (event) => {
        const target = event.target;
        if (
          target instanceof HTMLInputElement &&
          target.getAttribute("type") === "file"
        ) {
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
            this.ipc.postMessage(message), event.preventDefault();
          }
        }
      }),
      (this.ipc = window.ipc);
    const handler = (event) => this.handleEvent(event, event.type, !0);
    super.initialize(root, handler);
  }
  serializeIpcMessage(method, params = {}) {
    return JSON.stringify({ method, params });
  }
  scrollTo(id, behavior) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) node.scrollIntoView({ behavior });
  }
  getScrollHeight(id) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) return node.scrollHeight;
  }
  getScrollLeft(id) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) return node.scrollLeft;
  }
  getScrollTop(id) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) return node.scrollTop;
  }
  getScrollWidth(id) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement) return node.scrollWidth;
  }
  getClientRect(id) {
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
  setFocus(id, focus) {
    const node = this.nodes[id];
    if (node instanceof HTMLElement)
      if (focus) node.focus();
      else node.blur();
  }
  loadChild(array) {
    let node = this.stack[this.stack.length - 1];
    for (let i = 0; i < array.length; i++) {
      let end = array[i];
      for (node = node.firstChild; end > 0; end--) node = node.nextSibling;
    }
    return node;
  }
  appendChildren(id, many) {
    const root = this.nodes[id],
      els = this.stack.splice(this.stack.length - many);
    for (let k = 0; k < many; k++) root.appendChild(els[k]);
  }
  handleEvent(event, name, bubbles) {
    const target = event.target,
      realId = getTargetId(target),
      contents = serializeEvent(event, target);
    let body = { name, data: contents, element: realId, bubbles };
    if ((this.preventDefaults(event, target), this.liveview)) {
      if (
        target instanceof HTMLInputElement &&
        (event.type === "change" || event.type === "input")
      ) {
        if (target.getAttribute("type") === "file")
          this.readFiles(target, contents, bubbles, realId, name);
      }
    } else {
      const message = this.serializeIpcMessage("user_event", body);
      this.ipc.postMessage(message);
    }
  }
  preventDefaults(event, target) {
    let preventDefaultRequests = null;
    if (target instanceof Element)
      preventDefaultRequests = target.getAttribute("dioxus-prevent-default");
    if (
      preventDefaultRequests &&
      preventDefaultRequests.includes(`on${event.type}`)
    )
      event.preventDefault();
    if (event.type === "submit") event.preventDefault();
    if (target instanceof Element && event.type === "click")
      this.handleClickNavigate(event, target, preventDefaultRequests);
  }
  handleClickNavigate(event, target, preventDefaultRequests) {
    if (!this.intercept_link_redirects) return;
    if (target.tagName === "BUTTON" && event.type == "submit")
      event.preventDefault();
    let a_element = target.closest("a");
    if (a_element == null) return;
    event.preventDefault();
    let elementShouldPreventDefault =
        preventDefaultRequests && preventDefaultRequests.includes("onclick"),
      aElementShouldPreventDefault = a_element.getAttribute(
        "dioxus-prevent-default"
      ),
      linkShouldPreventDefault =
        aElementShouldPreventDefault &&
        aElementShouldPreventDefault.includes("onclick");
    if (!elementShouldPreventDefault && !linkShouldPreventDefault) {
      const href = a_element.getAttribute("href");
      if (href !== "" && href !== null && href !== void 0)
        this.ipc.postMessage(
          this.serializeIpcMessage("browser_open", { href })
        );
    }
  }
  enqueueBytes(bytes) {
    this.queuedBytes.push(bytes);
  }
  flushQueuedBytes() {
    const byteArray = this.queuedBytes;
    this.queuedBytes = [];
    for (let bytes of byteArray) this.run_from_bytes(bytes);
  }
  rafEdits(headless, bytes) {
    if (headless) this.run_from_bytes(bytes), this.waitForRequest(headless);
    else
      this.enqueueBytes(bytes),
        requestAnimationFrame(() => {
          this.flushQueuedBytes(), this.waitForRequest(headless);
        });
  }
  waitForRequest(headless) {
    fetch(new Request(this.editsPath))
      .then((response) => response.arrayBuffer())
      .then((bytes) => {
        this.rafEdits(headless, bytes);
      });
  }
  kickAllStylesheetsOnPage() {
    let stylesheets = document.querySelectorAll("link[rel=stylesheet]");
    for (let i = 0; i < stylesheets.length; i++) {
      let sheet = stylesheets[i];
      fetch(sheet.href, { cache: "reload" }).then(() => {
        sheet.href = sheet.href + "?" + Math.random();
      });
    }
  }
  async readFiles(target, contents, bubbles, realId, name) {
    let files = target.files,
      file_contents = {};
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      file_contents[file.name] = Array.from(
        new Uint8Array(await file.arrayBuffer())
      );
    }
    contents.files = { files: file_contents };
    const message = this.serializeIpcMessage("user_event", {
      name,
      element: realId,
      data: contents,
      bubbles,
    });
    this.ipc.postMessage(message);
  }
}
export { NativeInterpreter };
