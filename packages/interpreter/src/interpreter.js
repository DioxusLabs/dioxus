export function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.interpreter = new Interpreter(root);
    window.ipc.postMessage(serializeIpcMessage("initialize"));
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

export class JsInterpreter {
  constructor(root, mem, _ptr_ptr, _len_ptr) {
    this.root = root;
    this.lastNode = root;
    this.listeners = new ListenerMap(root);
    this.handlers = {};
    this.handler = () => { };
    this.nodes = [root];
    this.parents = [];
    this.u8Buf = new Uint8Array(mem.buffer);
    this.ptr_ptr = _ptr_ptr;
    this.len_ptr = _len_ptr;
  }

  SetEventHandler(handler) {
    this.handler = handler;
  }

  Work() {
    const u8Buf = this.u8Buf;
    this.u8BufPos = this.decodePtr(this.ptr_ptr);
    const end = this.u8BufPos + this.decodePtr(this.len_ptr);
    while (this.u8BufPos < end) {
      switch (u8Buf[this.u8BufPos++]) {
        // append children
        case 0:
          {
            const parent = this.nodes[this.decodeId()];
            const len = this.decodeU32();
            for (let i = 0; i < len; i++) {
              parent.appendChild(this.nodes[this.decodeId()]);
            }
          }
          break;
        // replace with
        case 1:
          {
            const parent = this.nodes[this.decodeId()];
            const len = this.decodeU32();
            const children = [];
            for (let i = 0; i < len; i++) {
              children.push(this.nodes[this.decodeId()]);
            }
            parent.replaceWith(...children);
          }
          break;
        // insert after
        case 2:
          {
            const parent = this.nodes[this.decodeId()];
            const len = this.decodeU32();
            const children = [];
            for (let i = 0; i < len; i++) {
              children.push(this.nodes[this.decodeId()]);
            }
            parent.after(...children);
          }
          break;
        // insert before
        case 3:
          {
            const parent = this.nodes[this.decodeId()];
            const len = this.decodeU32();
            const children = [];
            for (let i = 0; i < len; i++) {
              children.push(this.nodes[this.decodeId()]);
            }
            parent.before(...children);
          }
          break;
        // remove
        case 4:
          {
            const id = this.decodeId();
            this.nodes[id].remove();
          }
          break;
        // create text node
        case 5:
          {
            const id = this.decodeId();
            const str_len = this.decodeU32();
            this.lastNode = document.createTextNode(this.utf8Decode(str_len));
            if (id !== 0) {
              globalData2.nodes[id - 1] = this.lastNode;
            }
          }
          break;
        // create element
        case 6:
          this.lastNode = this.CreateFullElement();
          break;
        // create placeholder
        case 7:
          {
            const id = this.decodeId();
            this.lastNode = document.createElement("pre");
            this.lastNode.hidden = true;
            globalData2.nodes[id - 1] = this.lastNode;
          }
          break;
        // new event listener
        case 8:
          {
            const id = this.decodeId();
            const event = u8Buf[this.u8BufPos++];
            if (event === 255) {
              const len = this.decodeU32();
              str = this.asciiDecode(len);
            }
            else {
              str = convertEvent(event);
            }
            let bubbles = u8Buf[this.u8BufPos++] == 0;
            this.NewEventListener(event, id, bubbles);
          }
          break;
        // remove event listener
        case 9:
          {
            const id = this.decodeId();
            const event = u8Buf[this.u8BufPos++];
            if (event === 255) {
              const len = this.decodeU32();
              str = this.asciiDecode(start, len);
            }
            else {
              str = convertEvent(event);
            }
            let bubbles = u8Buf[this.u8BufPos++] == 0;
            this.RemoveEventListener(event, id, bubbles);
            console.log("todo");
          }
          break;
        // set text
        case 10:
          {
            const id = this.decodeId();
            const str_len = this.decodeU32();
            const text = utf8Decode(str_len);
            if (id === 0) {
              this.lastNode.textContent = text;
            }
            else {
              const node = this.nodes[id];
              node.textContent = text;
            }
          }
          break;
        // set attribute
        case 11:
          {
            const id = this.decodeId();
            const attr = decodeAttribute();
            const val = decodeValue();
            if (id === 0) {
              this.lastNode.setAttribute(attr, val);
            }
            else {
              this.nodes[id].setAttribute(attr, val);
            }
          }
          break;
        // remove attribute
        case 12:
          {
            let attr;
            const id = this.decodeId();
            {
              const len = this.decodeU32();
              attr = this.asciiDecode(len);
            }
            let len = this.decodeU32();
            const ns = this.asciiDecode(len);
            if (id === 0) {
              this.lastNode.removeAttributeNS(ns, attr);
            } else {
              this.nodes[id].removeAttributeNS(ns, attr);
            }
          }
          break;
        // clone node
        case 13:
          {
            const toCloneId = this.decodeId();
            const toStoreId = this.decodeId();
            this.lastNode = this.nodes[toCloneId].cloneNode(true);
            if (toStoreId !== 0) {
              this.nodes[toStoreId - 1] = this.lastNode;
            }
          }
          break;
        // clone node children
        case 14:
          {
            const toCloneId = this.decodeId();
            const childrenIds = u8Buf[this.u8BufPos++];
            let parent = this.nodes[toCloneId].cloneNode(true);
            let i = 0;
            for (let current = parent.firstChild; current !== null; current = current.nextSibling) {
              this.nodes[this.decodeId() - 1] = current;
              i++;
            }
          }
          break;
        // first child
        case 15:
          {
            this.lastNode = this.lastNode.firstChild;
          }
          break;
        // next sibling
        case 16:
          {
            this.lastNode = this.lastNode.nextSibling;
          }
          break;
        // parent
        case 17:
          {
            this.lastNode = this.lastNode.parentNode;
          }
          break;
        // store with id
        case 18:
          {
            const id = this.this.decodeId();
            this.nodes[id - 1] = this.lastNode;
          }
          break;
        // set last node
        case 19:
          {
            const id = this.this.decodeId();
            this.lastNode = this.this.nodes[id];
          }
          break;
        default:
          this.u8BufPos--;
          console.log(`unknown opcode ${u8Buf[this.u8BufPos]}`);
          return;
      }
    }
  }

  checkAppendParent() {
    if (this.parents.length > 0) {
      const lastParent = this.parents[this.parents.length - 1];
      lastParent[1]--;
      if (lastParent[1] === 0) {
        this.parents.pop();
      }
      lastParent[0].appendChild(this.lastNode);
    }
  }

  utf8Decode(byteLength) {
    const end = this.u8BufPos + byteLength;
    let out = "";
    while (this.u8BufPos < end) {
      const byte1 = this.u8Buf[this.u8BufPos++];
      if ((byte1 & 0x80) === 0) {
        // 1 byte
        out += String.fromCharCode(byte1);
      } else if ((byte1 & 0xe0) === 0xc0) {
        // 2 bytes
        const byte2 = this.u8Buf[this.u8BufPos++] & 0x3f;
        out += String.fromCharCode(((byte1 & 0x1f) << 6) | byte2);
      } else if ((byte1 & 0xf0) === 0xe0) {
        // 3 bytes
        const byte2 = this.u8Buf[this.u8BufPos++] & 0x3f;
        const byte3 = this.u8Buf[this.u8BufPos++] & 0x3f;
        out += String.fromCharCode(((byte1 & 0x1f) << 12) | (byte2 << 6) | byte3);
      } else if ((byte1 & 0xf8) === 0xf0) {
        // 4 bytes
        const byte2 = this.u8Buf[this.u8BufPos++] & 0x3f;
        const byte3 = this.u8Buf[this.u8BufPos++] & 0x3f;
        const byte4 = this.u8Buf[this.u8BufPos++] & 0x3f;
        let unit = ((byte1 & 0x07) << 0x12) | (byte2 << 0x0c) | (byte3 << 0x06) | byte4;
        if (unit > 0xffff) {
          unit -= 0x10000;
          out += String.fromCharCode(((unit >>> 10) & 0x3ff) | 0xd800);
          unit = 0xdc00 | (unit & 0x3ff);
        }
        out += String.fromCharCode(unit);
      } else {
        out += String.fromCharCode(byte1);
      }
    }

    return out;
  }

  asciiDecode(byteLength) {
    const end = this.u8BufPos + byteLength;
    let out = "";
    while (this.u8BufPos < end) {
      out += String.fromCharCode(this.u8Buf[this.u8BufPos++]);
    }
    return out;
  }

  decodeId() {
    const id_code = this.u8Buf[this.u8BufPos++];
    if (id_code === 0) {
      return 0;
    }
    else if (id_code === 1) {
      let id = this.u8Buf[this.u8BufPos++];
      for (let i = 1; i < 8; i++) {
        id |= this.u8Buf[this.u8BufPos++] << (i * 8);
      }
      return id;
    }
  }

  decodePtr(_start) {
    let start = _start;
    const u8Buf = this.u8Buf;
    let val = u8Buf[start++];
    for (let i = 1; i < 4; i++) {
      val |= u8Buf[start++] << (i * 8);
    }
    return val;
  }

  decodeU32() {
    let val = this.u8Buf[this.u8BufPos++];
    for (let i = 1; i < 4; i++) {
      val |= this.u8Buf[this.u8BufPos++] << (i * 8);
    }
    return val;
  }

  CreateElement() {
    const len = this.decodeU32();
    const str = this.asciiDecode(len);

    const ns_len = this.decodeU32();
    if (ns_len === 0) {
      return document.createElement(str);
    }
    else {
      const ns = this.asciiDecode(ns_len);
      return document.createElementNS(str, ns);
    }
  }

  CreateFullElement() {
    const id = this.decodeId();
    this.lastNode = this.CreateElement();
    this.checkAppendParent();
    if (id !== 0) {
      this.nodes[id - 1] = element;
    }
    const children = this.decodeU32();
    this.parents.push([this.lastNode, children]);
    return element;
  }
  AppendChildren(root, children) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    for (let i = 0; i < children.length; i++) {
      node.appendChild(this.nodes[children[i]]);
    }
  }
  ReplaceWith(root, nodes) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    let els = [];
    for (let i = 0; i < nodes.length; i++) {
      els.push(this.nodes[nodes[i]])
    }
    node.replaceWith(...els);
  }
  InsertAfter(root, nodes) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    let els = [];
    for (let i = 0; i < nodes.length; i++) {
      els.push(this.nodes[nodes[i]])
    }
    node.after(...els);
  }
  InsertBefore(root, nodes) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    let els = [];
    for (let i = 0; i < nodes.length; i++) {
      els.push(this.nodes[nodes[i]])
    }
    node.before(...els);
  }
  Remove(root) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    if (node !== undefined) {
      node.remove();
    }
  }
  CreateTextNode(text, root) {
    this.lastNode = document.createTextNode(text);
    this.checkAppendParent();
    if (root !== null) {
      this.nodes[root] = this.lastNode;
    }
  }
  CreatePlaceholder(root) {
    this.lastNode = document.createElement("pre");
    this.lastNode.hidden = true;
    this.checkAppendParent();
    if (root !== null) {
      this.nodes[root] = this.lastNode;
    }
  }
  NewEventListener(event_name, root, bubbles) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    node.setAttribute("data-dioxus-id", `${root}`);
    this.listeners.create(event_name, node, this.handler, bubbles);
  }
  RemoveEventListener(event_name, root, bubbles) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    node.removeAttribute(`data-dioxus-id`);
    this.listeners.remove(node, event_name, bubbles);
  }
  SetText(root, text) {
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
    node.data = text;
  }
  SetAttribute(root, field, value, ns) {
    const name = field;
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
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
    let node;
    if (root === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[root];
    }
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
  CloneNode(old, new_id) {
    let node;
    if (old === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[old];
    }
    this.nodes[new_id] = node.cloneNode(true);
  }
  CloneNodeChildren(old, new_ids) {
    let node;
    if (old === null) {
      node = this.lastNode;
    } else {
      node = this.nodes[old];
    }
    const old_node = node.cloneNode(true);
    let i = 0;
    for (let node = old_node.firstChild; i < new_ids.length; node = node.nextSibling) {
      this.nodes[new_ids[i++]] = node;
    }
  }
  FirstChild() {
    this.lastNode = this.lastNode.firstChild;
  }
  NextSibling() {
    this.lastNode = this.lastNode.nextSibling;
  }
  ParentNode() {
    this.lastNode = this.lastNode.parentNode;
  }
  StoreWithId(id) {
    this.nodes[id] = this.lastNode;
  }
  SetLastNode(root) {
    this.lastNode = this.nodes[root];
  }
  handleEdits(edits) {
    for (let edit of edits) {
      this.handleEdit(edit);
    }
  }
  handleEdit(edit) {
    switch (edit.type) {
      case "PushRoot":
        this.PushRoot(edit.root);
        break;
      case "AppendChildren":
        this.AppendChildren(edit.root, edit.children);
        break;
      case "ReplaceWith":
        this.ReplaceWith(edit.root, edit.nodes);
        break;
      case "InsertAfter":
        this.InsertAfter(edit.root, edit.nodes);
        break;
      case "InsertBefore":
        this.InsertBefore(edit.root, edit.nodes);
        break;
      case "Remove":
        this.Remove(edit.root);
        break;
      case "CreateTextNode":
        this.CreateTextNode(edit.text, edit.root);
        break;
      case "CreateElement":
        this.CreateElement(edit.tag, edit.root, edit.children);
        break;
      case "CreateElementNs":
        this.CreateElementNs(edit.tag, edit.root, edit.ns, edit.children);
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
            realId = parseInt(realId);
            window.ipc.postMessage(
              serializeIpcMessage("user_event", {
                event: edit.event_name,
                mounted_dom_id: realId,
                contents: contents,
              })
            );
          }
        };
        this.NewEventListener(edit.event_name, edit.root, handler, event_bubbles(edit.event_name));

        break;
      case "SetText":
        this.SetText(edit.root, edit.text);
        break;
      case "SetAttribute":
        this.SetAttribute(edit.root, edit.field, edit.value, edit.ns);
        break;
      case "RemoveAttribute":
        this.RemoveAttribute(edit.root, edit.name, edit.ns);
        break;
      case "CloneNode":
        this.CloneNode(edit.id, edit.new_id);
        break;
      case "CloneNodeChildren":
        this.CloneNodeChildren(edit.id, edit.new_ids);
        break;
      case "FirstChild":
        this.FirstChild();
        break;
      case "NextSibling":
        this.NextSibling();
        break;
      case "ParentNode":
        this.ParentNode();
        break;
      case "StoreWithId":
        this.StoreWithId(BigInt(edit.id));
        break;
      case "SetLastNode":
        this.SetLastNode(BigInt(edit.id));
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