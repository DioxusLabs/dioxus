import { setAttributeInner } from "./common.js";

class ListenerMap {
  constructor(root) {
    // bubbling events can listen at the root element
    this.global = {};
    // non bubbling events listen at the element the listener was created at
    this.local = {};
    this.root = root;
  }

  create(event_name, element, handler, bubbles) {
    if (bubbles) {
      if (this.global[event_name] === undefined) {
        this.global[event_name] = {};
        this.global[event_name].active = 1;
        this.global[event_name].callback = handler;
        this.root.addEventListener(event_name, handler);
      } else {
        this.global[event_name].active++;
      }
    } else {
      const id = element.getAttribute("data-dioxus-id");
      if (!this.local[id]) {
        this.local[id] = {};
      }
      this.local[id][event_name] = handler;
      element.addEventListener(event_name, handler);
    }
  }

  remove(element, event_name, bubbles) {
    if (bubbles) {
      this.global[event_name].active--;
      if (this.global[event_name].active === 0) {
        this.root.removeEventListener(
          event_name,
          this.global[event_name].callback
        );
        delete this.global[event_name];
      }
    } else {
      const id = element.getAttribute("data-dioxus-id");
      delete this.local[id][event_name];
      if (this.local[id].length === 0) {
        delete this.local[id];
      }
      element.removeEventListener(event_name, handler);
    }
  }

  removeAllNonBubbling(element) {
    const id = element.getAttribute("data-dioxus-id");
    delete this.local[id];
  }
}

class InterpreterConfig {
  constructor(intercept_link_redirects) {
    this.intercept_link_redirects = intercept_link_redirects;
  }
}

class Interpreter {
  constructor(root, config) {
    this.config = config;
    this.root = root;
    this.listeners = new ListenerMap(root);
    this.nodes = [root];
    this.stack = [root];
    this.handlers = {};
    this.templates = {};
    this.lastNodeWasText = false;
  }
  top() {
    return this.stack[this.stack.length - 1];
  }
  pop() {
    return this.stack.pop();
  }
  MountToRoot() {
    this.AppendChildren(this.stack.length - 1);
  }
  SetNode(id, node) {
    this.nodes[id] = node;
  }
  PushRoot(root) {
    const node = this.nodes[root];
    this.stack.push(node);
  }
  PopRoot() {
    this.stack.pop();
  }
  AppendChildren(many) {
    // let root = this.nodes[id];
    let root = this.stack[this.stack.length - 1 - many];
    let to_add = this.stack.splice(this.stack.length - many);
    for (let i = 0; i < many; i++) {
      root.appendChild(to_add[i]);
    }
  }
  ReplaceWith(root_id, m) {
    let root = this.nodes[root_id];
    let els = this.stack.splice(this.stack.length - m);
    if (is_element_node(root.nodeType)) {
      this.listeners.removeAllNonBubbling(root);
    }
    root.replaceWith(...els);
  }
  InsertAfter(root, n) {
    let old = this.nodes[root];
    let new_nodes = this.stack.splice(this.stack.length - n);
    old.after(...new_nodes);
  }
  InsertBefore(root, n) {
    let old = this.nodes[root];
    let new_nodes = this.stack.splice(this.stack.length - n);
    old.before(...new_nodes);
  }
  Remove(root) {
    let node = this.nodes[root];
    if (node !== undefined) {
      if (is_element_node(node)) {
        this.listeners.removeAllNonBubbling(node);
      }
      node.remove();
    }
  }
  CreateTextNode(text, root) {
    const node = document.createTextNode(text);
    this.nodes[root] = node;
    this.stack.push(node);
  }
  CreatePlaceholder(root) {
    let el = document.createElement("pre");
    el.hidden = true;
    this.stack.push(el);
    this.nodes[root] = el;
  }
  NewEventListener(event_name, root, bubbles, handler) {
    const element = this.nodes[root];
    element.setAttribute("data-dioxus-id", `${root}`);
    this.listeners.create(event_name, element, handler, bubbles);
  }
  RemoveEventListener(root, event_name, bubbles) {
    const element = this.nodes[root];
    element.removeAttribute(`data-dioxus-id`);
    this.listeners.remove(element, event_name, bubbles);
  }
  SetText(root, text) {
    this.nodes[root].textContent = text;
  }
  SetAttribute(id, field, value, ns) {
    if (value === null) {
      this.RemoveAttribute(id, field, ns);
    } else {
      const node = this.nodes[id];
      setAttributeInner(node, field, value, ns);
    }
  }
  RemoveAttribute(root, field, ns) {
    const node = this.nodes[root];
    if (!ns) {
      switch (field) {
        case "value":
          node.value = "";
          break;
        case "checked":
          node.checked = false;
          break;
        case "selected":
          node.selected = false;
          break;
        case "dangerous_inner_html":
          node.innerHTML = "";
          break;
        default:
          node.removeAttribute(field);
          break;
      }
    } else if (ns == "style") {
      node.style.removeProperty(name);
    } else {
      node.removeAttributeNS(ns, field);
    }
  }

  GetClientRect(id) {
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

  ScrollTo(id, behavior) {
    const node = this.nodes[id];
    if (!node) {
      return false;
    }
    node.scrollIntoView({
      behavior: behavior,
    });
    return true;
  }

  /// Set the focus on the element
  SetFocus(id, focus) {
    const node = this.nodes[id];
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

  handleEdits(edits) {
    for (let template of edits.templates) {
      this.SaveTemplate(template);
    }

    for (let edit of edits.edits) {
      this.handleEdit(edit);
    }

    /*POST_HANDLE_EDITS*/
  }

  SaveTemplate(template) {
    let roots = [];
    for (let root of template.roots) {
      roots.push(this.MakeTemplateNode(root));
    }
    this.templates[template.name] = roots;
  }

  MakeTemplateNode(node) {
    switch (node.type) {
      case "Text":
        return document.createTextNode(node.text);
      case "Dynamic":
        let dyn = document.createElement("pre");
        dyn.hidden = true;
        return dyn;
      case "DynamicText":
        return document.createTextNode("placeholder");
      case "Element":
        let el;

        if (node.namespace != null) {
          el = document.createElementNS(node.namespace, node.tag);
        } else {
          el = document.createElement(node.tag);
        }

        for (let attr of node.attrs) {
          if (attr.type == "Static") {
            setAttributeInner(el, attr.name, attr.value, attr.namespace);
          }
        }

        for (let child of node.children) {
          el.appendChild(this.MakeTemplateNode(child));
        }

        return el;
    }
  }
  AssignId(path, id) {
    this.nodes[id] = this.LoadChild(path);
  }
  LoadChild(path) {
    // iterate through each number and get that child
    let node = this.stack[this.stack.length - 1];

    for (let i = 0; i < path.length; i++) {
      node = node.childNodes[path[i]];
    }

    return node;
  }
  HydrateText(path, value, id) {
    let node = this.LoadChild(path);

    if (node.nodeType == Node.TEXT_NODE) {
      node.textContent = value;
    } else {
      // replace with a textnode
      let text = document.createTextNode(value);
      node.replaceWith(text);
      node = text;
    }

    this.nodes[id] = node;
  }
  ReplacePlaceholder(path, m) {
    let els = this.stack.splice(this.stack.length - m);
    let node = this.LoadChild(path);
    node.replaceWith(...els);
  }
  LoadTemplate(name, index, id) {
    let node = this.templates[name][index].cloneNode(true);
    this.nodes[id] = node;
    this.stack.push(node);
  }
  handleEdit(edit) {
    switch (edit.type) {
      case "AppendChildren":
        this.AppendChildren(edit.m);
        break;
      case "AssignId":
        this.AssignId(edit.path, edit.id);
        break;
      case "CreatePlaceholder":
        this.CreatePlaceholder(edit.id);
        break;
      case "CreateTextNode":
        this.CreateTextNode(edit.value, edit.id);
        break;
      case "HydrateText":
        this.HydrateText(edit.path, edit.value, edit.id);
        break;
      case "LoadTemplate":
        this.LoadTemplate(edit.name, edit.index, edit.id);
        break;
      case "PushRoot":
        this.PushRoot(edit.id);
        break;
      case "ReplaceWith":
        this.ReplaceWith(edit.id, edit.m);
        break;
      case "ReplacePlaceholder":
        this.ReplacePlaceholder(edit.path, edit.m);
        break;
      case "InsertAfter":
        this.InsertAfter(edit.id, edit.m);
        break;
      case "InsertBefore":
        this.InsertBefore(edit.id, edit.m);
        break;
      case "Remove":
        this.Remove(edit.id);
        break;
      case "SetText":
        this.SetText(edit.id, edit.value);
        break;
      case "SetAttribute":
        this.SetAttribute(edit.id, edit.name, edit.value, edit.ns);
        break;
      case "RemoveAttribute":
        this.RemoveAttribute(edit.id, edit.name, edit.ns);
        break;
      case "RemoveEventListener":
        this.RemoveEventListener(edit.id, edit.name);
        break;
      case "NewEventListener":
        let bubbles = event_bubbles(edit.name);

        // if this is a mounted listener, we send the event immediately
        if (edit.name === "mounted") {
          window.ipc.postMessage(
            serializeIpcMessage("user_event", {
              name: edit.name,
              element: edit.id,
              data: null,
              bubbles,
            })
          );
        } else {
          this.NewEventListener(edit.name, edit.id, bubbles, (event) => {
            handler(event, edit.name, bubbles, this.config);
          });
        }
        break;
    }
  }
}

// this handler is only provided on the desktop and liveview implementations since this
// method is not used by the web implementation
function handler(event, name, bubbles, config) {
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
                serializeIpcMessage("browser_open", { href })
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

    let contents = serialize_event(event);

    /*POST_EVENT_SERIALIZATION*/

    if (
      target.tagName === "FORM" &&
      (event.type === "submit" || event.type === "input")
    ) {
      const formData = new FormData(target);

      for (let name of formData.keys()) {
        let value = formData.getAll(name);
        contents.values[name] = value;
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
      serializeIpcMessage("user_event", {
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

function serialize_event(event) {
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
      return { mouse: get_mouse_data(event) };
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
function serializeIpcMessage(method, params = {}) {
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
