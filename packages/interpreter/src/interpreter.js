// id > Number.MAX_SAFE_INTEGER/2 in template ref
// id <= Number.MAX_SAFE_INTEGER/2 in global nodes
const templateIdLimit = BigInt((Number.MAX_SAFE_INTEGER - 1) / 2);

export function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.ipc.postMessage(serializeIpcMessage("initialize"));
  }
}

class TemplateRef {
  constructor(fragment, dynamicNodePaths, roots, id) {
    this.fragment = fragment;
    this.dynamicNodePaths = dynamicNodePaths;
    this.roots = roots;
    this.id = id;
    this.placed = false;
    this.nodes = [];
  }

  build(id) {
    if (!this.nodes[id]) {
      let current = this.fragment;
      const path = this.dynamicNodePaths[id];
      for (let i = 0; i < path.length; i++) {
        const idx = path[i];
        current = current.firstChild;
        for (let i2 = 0; i2 < idx; i2++) {
          current = current.nextSibling;
        }
      }
      this.nodes[id] = current;
    }
  }

  get(id) {
    this.build(id);
    return this.nodes[id];
  }

  parent() {
    return this.roots[0].parentNode;
  }

  first() {
    return this.roots[0];
  }

  last() {
    return this.roots[this.roots.length - 1];
  }

  move() {
    // move the root nodes into a new template
    this.fragment = new DocumentFragment();
    for (let n of this.roots) {
      this.fragment.appendChild(n);
    }
  }

  getFragment() {
    if (!this.placed) {
      this.placed = true;
    }
    else {
      this.move();
    }
    return this.fragment;
  }
}

class Template {
  constructor(template_id, id) {
    this.nodes = [];
    this.dynamicNodePaths = [];
    this.template_id = template_id;
    this.id = id;
    this.template = document.createElement("template");
  }

  finalize(roots) {
    for (let i = 0; i < roots.length; i++) {
      let node = roots[i];
      let path = [i];
      const is_element = node.nodeType == 1;
      const locally_static = is_element && !node.hasAttribute("data-dioxus-dynamic");
      if (!locally_static) {
        this.dynamicNodePaths[node.tmplId] = [...path];
      }
      const traverse_children = is_element && !node.hasAttribute("data-dioxus-fully-static");
      if (traverse_children) {
        this.createIds(path, node);
      }
      this.template.content.appendChild(node);
    }
    document.head.appendChild(this.template);
  }

  createIds(path, root) {
    let i = 0;
    for (let node = root.firstChild; node != null; node = node.nextSibling) {
      let new_path = [...path, i];
      const is_element = node.nodeType == 1;
      const locally_static = is_element && !node.hasAttribute("data-dioxus-dynamic");
      if (!locally_static) {
        this.dynamicNodePaths[node.tmplId] = [...new_path];
      }
      const traverse_children = is_element && !node.hasAttribute("data-dioxus-fully-static");
      if (traverse_children) {
        this.createIds(new_path, node);
      }
      i++;
    }
  }

  ref(id) {
    const template = this.template.content.cloneNode(true);
    let roots = [];
    this.reconstructingRefrencesIndex = 0;
    for (let node = template.firstChild; node != null; node = node.nextSibling) {
      roots.push(node);
    }
    let ref = new TemplateRef(template, this.dynamicNodePaths, roots, id);
    // resolve ids for any nodes that can change
    for (let i = 0; i < this.dynamicNodePaths.length; i++) {
      if (this.dynamicNodePaths[i]) {
        ref.build(i);
      }
    }
    return ref;
  }
}

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
    }
    else {
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
      element.removeEventListener(event_name, handler);
    }
  }
}

export class Interpreter {
  constructor(root) {
    this.root = root;
    this.stack = [root];
    this.templateInProgress = null;
    this.insideTemplateRef = [];
    this.listeners = new ListenerMap(root);
    this.handlers = {};
    this.nodes = [root];
    this.templates = [];
  }
  top() {
    return this.stack[this.stack.length - 1];
  }
  pop() {
    return this.stack.pop();
  }
  currentTemplateId() {
    if (this.insideTemplateRef.length) {
      return this.insideTemplateRef[this.insideTemplateRef.length - 1].id;
    }
    else {
      return null;
    }
  }
  getId(id) {
    if (this.templateInProgress !== null) {
      return this.templates[this.templateInProgress].nodes[id - templateIdLimit];
    }
    else if (this.insideTemplateRef.length && id >= templateIdLimit) {
      return this.insideTemplateRef[this.insideTemplateRef.length - 1].get(id - templateIdLimit);
    }
    else {
      return this.nodes[id];
    }
  }
  SetNode(id, node) {
    if (this.templateInProgress !== null) {
      id -= templateIdLimit;
      node.tmplId = id;
      this.templates[this.templateInProgress].nodes[id] = node;
    }
    else if (this.insideTemplateRef.length && id >= templateIdLimit) {
      id -= templateIdLimit;
      let last = this.insideTemplateRef[this.insideTemplateRef.length - 1];
      last.childNodes[id] = node;
      if (last.nodeCache[id]) {
        last.nodeCache[id] = node;
      }
    }
    else {
      this.nodes[id] = node;
    }
  }
  PushRoot(root) {
    const node = this.getId(root);
    this.stack.push(node);
  }
  PopRoot() {
    this.stack.pop();
  }
  AppendChildren(many) {
    let root = this.stack[this.stack.length - (1 + many)];
    let to_add = this.stack.splice(this.stack.length - many);
    for (let i = 0; i < many; i++) {
      const child = to_add[i];
      if (child instanceof TemplateRef) {
        root.appendChild(child.getFragment());
      }
      else {
        root.appendChild(child);
      }
    }
  }
  ReplaceWith(root_id, m) {
    let root = this.getId(root_id);
    if (root instanceof TemplateRef) {
      this.InsertBefore(root_id, m);
      this.Remove(root_id);
    }
    else {
      let els = this.stack.splice(this.stack.length - m).map(function (el) {
        if (el instanceof TemplateRef) {
          return el.getFragment();
        }
        else {
          return el;
        }
      });
      root.replaceWith(...els);
    }
  }
  InsertAfter(root, n) {
    const old = this.getId(root);
    const new_nodes = this.stack.splice(this.stack.length - n).map(function (el) {
      if (el instanceof TemplateRef) {
        return el.getFragment();
      }
      else {
        return el;
      }
    });
    if (old instanceof TemplateRef) {
      const last = old.last();
      last.after(...new_nodes);
    }
    else {
      old.after(...new_nodes);
    }
  }
  InsertBefore(root, n) {
    const old = this.getId(root);
    const new_nodes = this.stack.splice(this.stack.length - n).map(function (el) {
      if (el instanceof TemplateRef) {
        return el.getFragment();
      }
      else {
        return el;
      }
    });
    if (old instanceof TemplateRef) {
      const first = old.first();
      first.before(...new_nodes);
    }
    else {
      old.before(...new_nodes);
    }
  }
  Remove(root) {
    let node = this.getId(root);
    if (node !== undefined) {
      if (node instanceof TemplateRef) {
        for (let child of node.roots) {
          child.remove();
        }
      }
      else {
        node.remove();
      }
    }
  }
  CreateTextNode(text, root) {
    const node = document.createTextNode(text);
    this.stack.push(node);
    this.SetNode(root, node);
  }
  CreateElement(tag, root) {
    const el = document.createElement(tag);
    this.stack.push(el);
    this.SetNode(root, el);
  }
  CreateElementNs(tag, root, ns) {
    let el = document.createElementNS(ns, tag);
    this.stack.push(el);
    this.SetNode(root, el);
  }
  CreatePlaceholder(root) {
    let el = document.createElement("pre");
    el.hidden = true;
    this.stack.push(el);
    this.SetNode(root, el);
  }
  NewEventListener(event_name, root, handler, bubbles) {
    const element = this.getId(root);
    if (root >= templateIdLimit) {
      let currentTemplateRefId = this.currentTemplateId();
      root -= templateIdLimit;
      element.setAttribute("data-dioxus-id", `${currentTemplateRefId},${root}`);
    }
    else {
      element.setAttribute("data-dioxus-id", `${root}`);
    }
    this.listeners.create(event_name, element, handler, bubbles);
  }
  RemoveEventListener(root, event_name, bubbles) {
    const element = this.getId(root);
    element.removeAttribute(`data-dioxus-id`);
    this.listeners.remove(element, event_name, bubbles);
  }
  SetText(root, text) {
    this.getId(root).data = text;
  }
  SetAttribute(root, field, value, ns) {
    const name = field;
    const node = this.getId(root);
    if (ns === "style") {
      // @ts-ignore
      node.style[name] = value;
    } else if (ns != null || ns != undefined) {
      node.setAttributeNS(ns, name, value);
    } else {
      switch (name) {
        case "value":
          if (value !== node.value) {
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
          if (value === "false" && bool_attrs.hasOwnProperty(name)) {
            node.removeAttribute(name);
          } else {
            node.setAttribute(name, value);
          }
      }
    }
  }
  RemoveAttribute(root, field, ns) {
    const name = field;
    const node = this.getId(root);
    if (ns == "style") {
      node.style.removeProperty(name);
    } else if (ns !== null || ns !== undefined) {
      node.removeAttributeNS(ns, name);
    } else if (name === "value") {
      node.value = "";
    } else if (name === "checked") {
      node.checked = false;
    } else if (name === "selected") {
      node.selected = false;
    } else if (name === "dangerous_inner_html") {
      node.innerHTML = "";
    } else {
      node.removeAttribute(name);
    }
  }
  CreateTemplateRef(id, template_id) {
    const el = this.templates[template_id].ref(id);
    this.nodes[id] = el;
    this.stack.push(el);
  }
  CreateTemplate(template_id) {
    this.templateInProgress = template_id;
    this.templates[template_id] = new Template(template_id, 0);
  }
  FinishTemplate(many) {
    this.templates[this.templateInProgress].finalize(this.stack.splice(this.stack.length - many));
    this.templateInProgress = null;
  }
  EnterTemplateRef(id) {
    this.insideTemplateRef.push(this.nodes[id]);
  }
  ExitTemplateRef() {
    this.insideTemplateRef.pop();
  }
  handleEdits(edits) {
    for (let edit of edits) {
      this.handleEdit(edit);
    }
  }
  CreateElementTemplate(tag, root, locally_static, fully_static) {
    const el = document.createElement(tag);
    this.stack.push(el);
    this.SetNode(root, el);
    if (!locally_static)
      el.setAttribute("data-dioxus-dynamic", "true");
    if (fully_static)
      el.setAttribute("data-dioxus-fully-static", fully_static);
  }
  CreateElementNsTemplate(tag, root, ns, locally_static, fully_static) {
    const el = document.createElementNS(ns, tag);
    this.stack.push(el);
    this.SetNode(root, el);
    if (!locally_static)
      el.setAttribute("data-dioxus-dynamic", "true");
    if (fully_static)
      el.setAttribute("data-dioxus-fully-static", fully_static);
  }
  CreateTextNodeTemplate(text, root, locally_static) {
    const node = document.createTextNode(text);
    this.stack.push(node);
    this.SetNode(root, node);
  }
  CreatePlaceholderTemplate(root) {
    const el = document.createElement("pre");
    el.setAttribute("data-dioxus-dynamic", "true");
    el.hidden = true;
    this.stack.push(el);
    this.SetNode(root, el);
  }
  handleEdit(edit) {
    switch (edit.type) {
      case "PushRoot":
        this.PushRoot(BigInt(edit.root));
        break;
      case "AppendChildren":
        this.AppendChildren(edit.many);
        break;
      case "ReplaceWith":
        this.ReplaceWith(BigInt(edit.root), edit.m);
        break;
      case "InsertAfter":
        this.InsertAfter(BigInt(edit.root), edit.n);
        break;
      case "InsertBefore":
        this.InsertBefore(BigInt(edit.root), edit.n);
        break;
      case "Remove":
        this.Remove(BigInt(edit.root));
        break;
      case "CreateTextNode":
        this.CreateTextNode(edit.text, BigInt(edit.root));
        break;
      case "CreateElement":
        this.CreateElement(edit.tag, BigInt(edit.root));
        break;
      case "CreateElementNs":
        this.CreateElementNs(edit.tag, BigInt(edit.root), edit.ns);
        break;
      case "CreatePlaceholder":
        this.CreatePlaceholder(BigInt(edit.root));
        break;
      case "RemoveEventListener":
        this.RemoveEventListener(BigInt(edit.root), edit.event_name);
        break;
      case "NewEventListener":
        // this handler is only provided on desktop implementations since this
        // method is not used by the web implementation
        let handler = (event) => {
          let target = event.target;
          if (target != null) {
            let realId = target.getAttribute(`data-dioxus-id`);
            let shouldPreventDefault = target.getAttribute(
              `dioxus-prevent-default`
            );

            if (event.type === "click") {
              // todo call prevent default if it's the right type of event
              if (shouldPreventDefault !== `onclick`) {
                if (target.tagName === "A") {
                  event.preventDefault();
                  const href = target.getAttribute("href");
                  if (href !== "" && href !== null && href !== undefined) {
                    window.ipc.postMessage(
                      serializeIpcMessage("browser_open", { href })
                    );
                  }
                }
              }

              // also prevent buttons from submitting
              if (target.tagName === "BUTTON" && event.type == "submit") {
                event.preventDefault();
              }
            }
            // walk the tree to find the real element
            while (realId == null) {
              // we've reached the root we don't want to send an event
              if (target.parentElement === null) {
                return;
              }

              target = target.parentElement;
              realId = target.getAttribute(`data-dioxus-id`);
            }

            shouldPreventDefault = target.getAttribute(
              `dioxus-prevent-default`
            );

            let contents = serialize_event(event);

            if (shouldPreventDefault === `on${event.type}`) {
              event.preventDefault();
            }

            if (event.type === "submit") {
              event.preventDefault();
            }

            if (
              target.tagName === "FORM" &&
              (event.type === "submit" || event.type === "input")
            ) {
              for (let x = 0; x < target.elements.length; x++) {
                let element = target.elements[x];
                let name = element.getAttribute("name");
                if (name != null) {
                  if (element.getAttribute("type") === "checkbox") {
                    // @ts-ignore
                    contents.values[name] = element.checked ? "true" : "false";
                  } else if (element.getAttribute("type") === "radio") {
                    if (element.checked) {
                      contents.values[name] = element.value;
                    }
                  } else {
                    // @ts-ignore
                    contents.values[name] =
                      element.value ?? element.textContent;
                  }
                }
              }
            }

            if (realId === null) {
              return;
            }
            if (realId.includes(",")) {
              realId = realId.split(',');
              realId = {
                template_ref_id: parseInt(realId[0]),
                template_node_id: parseInt(realId[1]),
              };
            }
            else {
              realId = parseInt(realId);
            }
            window.ipc.postMessage(
              serializeIpcMessage("user_event", {
                event: edit.event_name,
                mounted_dom_id: realId,
                contents: contents,
              })
            );
          }
        };
        this.NewEventListener(edit.event_name, BigInt(edit.root), handler, event_bubbles(edit.event_name));

        break;
      case "SetText":
        this.SetText(BigInt(edit.root), edit.text);
        break;
      case "SetAttribute":
        this.SetAttribute(BigInt(edit.root), edit.field, edit.value, edit.ns);
        break;
      case "RemoveAttribute":
        this.RemoveAttribute(BigInt(edit.root), edit.name, edit.ns);
        break;
      case "PopRoot":
        this.PopRoot();
        break;
      case "CreateTemplateRef":
        this.CreateTemplateRef(BigInt(edit.id), edit.template_id);
        break;
      case "CreateTemplate":
        this.CreateTemplate(BigInt(edit.id));
        break;
      case "FinishTemplate":
        this.FinishTemplate(edit.len);
        break;
      case "EnterTemplateRef":
        this.EnterTemplateRef(BigInt(edit.root));
        break;
      case "ExitTemplateRef":
        this.ExitTemplateRef();
        break;
      case "CreateElementTemplate":
        this.CreateElementTemplate(edit.tag, BigInt(edit.root), edit.locally_static, edit.fully_static);
        break;
      case "CreateElementNsTemplate":
        this.CreateElementNsTemplate(edit.tag, BigInt(edit.root), edit.ns, edit.locally_static, edit.fully_static);
        break;
      case "CreateTextNodeTemplate":
        this.CreateTextNodeTemplate(edit.text, BigInt(edit.root), edit.locally_static);
        break;
      case "CreatePlaceholderTemplate":
        this.CreatePlaceholderTemplate(BigInt(edit.root));
        break;
    }
  }
}

export function serialize_event(event) {
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
    case "click":
    case "contextmenu":
    case "doubleclick":
    case "dblclick":
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
      return false;
    case "loadedmetadata":
      return false;
    case "loadstart":
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
  }
}