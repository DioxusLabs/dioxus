let m,p,ls,d,t,op,i,e,z,metaflags;
            ;

class InterpreterConfig {
  constructor(intercept_link_redirects) {
    this.intercept_link_redirects = intercept_link_redirects;
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
          const message = serializeIpcMessage("user_event", {
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
function SetAttributeInner(node, field, value, ns) {
  const name = field;
  if (ns === "style") {
    // ????? why do we need to do this
    if (node.style === undefined) {
      node.style = {};
    }
    node.style[name] = value;
  } else if (ns !== null && ns !== undefined && ns !== "") {
    node.setAttributeNS(ns, name, value);
  } else {
    switch (name) {
      case "value":
        if (value !== node.value) {
          node.value = value;
        }
        break;
      case "initial_value":
        node.defaultValue = value;
        break;
      case "checked":
        node.checked = truthy(value);
        break;
      case "selected":
        node.selected = truthy(value);
        break;
      case "dangerous_inner_html":
        node.innerHTML = value;
        break;
      default:
        // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
        if (!truthy(value) && bool_attrs.hasOwnProperty(name)) {
          node.removeAttribute(name);
        } else {
          node.setAttribute(name, value);
        }
    }
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
function initialize(root) {
  nodes = [root];
  stack = [root];
  listeners.root = root;
}
function AppendChildren(id, many) {
  root = nodes[id];
  els = stack.splice(stack.length - many);
  for (k = 0; k < many; k++) {
    root.appendChild(els[k]);
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
  webkitdirectory: true,
};
function truthy(val) {
  return val === "true" || val === true;
}


function getClientRect(id) {
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

function scrollTo(id, behavior) {
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
function setFocus(id, focus) {
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

function saveTemplate(template) {
  let roots = [];
  for (let root of template.roots) {
    roots.push(this.MakeTemplateNode(root));
  }
  this.templates[template.name] = roots;
}

function makeTemplateNode(node) {
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
let u32buf,u32bufp;let u8buf,u8bufp;let s = "";let lsp,sp,sl; let c = new TextDecoder();const ns_cache = [];
                    let ns_cache_cache_hit, ns_cache_cache_idx;
                    function get_ns_cache() {
                        ns_cache_cache_idx = u8buf[u8bufp++];
                        if(ns_cache_cache_idx & 128){
                            ns_cache_cache_hit=s.substring(sp,sp+=u8buf[u8bufp++]);
                            ns_cache[ns_cache_cache_idx&4294967167]=ns_cache_cache_hit;
                            return ns_cache_cache_hit;
                        }
                        else{
                            return ns_cache[ns_cache_cache_idx&4294967167];
                        }
                    }const attr = [];
                    let attr_cache_hit, attr_cache_idx;
                    function get_attr() {
                        attr_cache_idx = u8buf[u8bufp++];
                        if(attr_cache_idx & 128){
                            attr_cache_hit=s.substring(sp,sp+=u8buf[u8bufp++]);
                            attr[attr_cache_idx&4294967167]=attr_cache_hit;
                            return attr_cache_hit;
                        }
                        else{
                            return attr[attr_cache_idx&4294967167];
                        }
                    }const evt = [];
                    let evt_cache_hit, evt_cache_idx;
                    function get_evt() {
                        evt_cache_idx = u8buf[u8bufp++];
                        if(evt_cache_idx & 128){
                            evt_cache_hit=s.substring(sp,sp+=u8buf[u8bufp++]);
                            evt[evt_cache_idx&4294967167]=evt_cache_hit;
                            return evt_cache_hit;
                        }
                        else{
                            return evt[evt_cache_idx&4294967167];
                        }
                    }const namespace = [];
                    let namespace_cache_hit, namespace_cache_idx;
                    function get_namespace() {
                        namespace_cache_idx = u8buf[u8bufp++];
                        if(namespace_cache_idx & 128){
                            namespace_cache_hit=s.substring(sp,sp+=u8buf[u8bufp++]);
                            namespace[namespace_cache_idx&4294967167]=namespace_cache_hit;
                            return namespace_cache_hit;
                        }
                        else{
                            return namespace[namespace_cache_idx&4294967167];
                        }
                    }const el = [];
                    let el_cache_hit, el_cache_idx;
                    function get_el() {
                        el_cache_idx = u8buf[u8bufp++];
                        if(el_cache_idx & 128){
                            el_cache_hit=s.substring(sp,sp+=u8buf[u8bufp++]);
                            el[el_cache_idx&4294967167]=el_cache_hit;
                            return el_cache_hit;
                        }
                        else{
                            return el[el_cache_idx&4294967167];
                        }
                    }let u16buf,u16bufp;
            let bubbles,field,value,array,event_name,many,id,ns;
             function create(r){
                d=r;
            }
             function update_memory(b){
                m=new DataView(b.buffer)
            }
             function run(){
                metaflags=m.getUint32(d,true);
                if((metaflags>>>6)&1){
                    ls=m.getUint32(d+6*4,true);
                }
                p=ls;
                if ((metaflags>>>3)&1){
                t = m.getUint32(d+3*4,true);
                u32buf=new Uint32Array(m.buffer,t,((m.buffer.byteLength-t)-(m.buffer.byteLength-t)%4)/4);
            }
            u32bufp=0;if ((metaflags>>>5)&1){
                t = m.getUint32(d+5*4,true);
                u8buf=new Uint8Array(m.buffer,t,((m.buffer.byteLength-t)-(m.buffer.byteLength-t)%1)/1);
            }
            u8bufp=0;if (metaflags&1){
                lsp = m.getUint32(d+1*4,true);
            }
            if ((metaflags>>>2)&1) {
                sl = m.getUint32(d+2*4,true);
                if ((metaflags>>>1)&1) {
                    sp = lsp;
                    s = "";
                    e = sp + ((sl / 4) | 0) * 4;
                    while (sp < e) {
                        t = m.getUint32(sp, true);
                        s += String.fromCharCode(
                            t & 255,
                            (t & 65280) >> 8,
                            (t & 16711680) >> 16,
                            t >> 24
                        );
                        sp += 4;
                    }
                    while (sp < lsp + sl) {
                        s += String.fromCharCode(m.getUint8(sp++));
                    }
                } else {
                    s = c.decode(new DataView(m.buffer, lsp, sl));
                }
            }
            sp=0;if ((metaflags>>>4)&1){
                t = m.getUint32(d+4*4,true);
                u16buf=new Uint16Array(m.buffer,t,((m.buffer.byteLength-t)-(m.buffer.byteLength-t)%2)/2);
            }
            u16bufp=0;
                for(;;){
                    op=m.getUint32(p,true);
                    p+=4;
                    z=0;
                    while(z++<4){
                        switch(op&255){
                            case 0:{AppendChildren(root, stack.length-1);}break;case 1:{stack.push(nodes[u32buf[u32bufp++]]);}break;case 2:{AppendChildren(u32buf[u32bufp++], u16buf[u16bufp++]);}break;case 3:many=u16buf[u16bufp++];{
            root = stack[stack.length-many-1];
            els = stack.splice(stack.length-many);
            for (k = 0; k < many; k++) {
                root.appendChild(els[k]);
            }
        }break;case 4:{stack.pop();}break;case 5:{root = nodes[u32buf[u32bufp++]]; els = stack.splice(stack.length-u16buf[u16bufp++]); if (root.listening) { listeners.removeAllNonBubbling(root); } root.replaceWith(...els);}break;case 6:{nodes[u32buf[u32bufp++]].after(...stack.splice(stack.length-u16buf[u16bufp++]));}break;case 7:{nodes[u32buf[u32bufp++]].before(...stack.splice(stack.length-u16buf[u16bufp++]));}break;case 8:{node = nodes[u32buf[u32bufp++]]; if (node !== undefined) { if (node.listening) { listeners.removeAllNonBubbling(node); } node.remove(); }}break;case 9:{stack.push(document.createTextNode(s.substring(sp,sp+=u32buf[u32bufp++])));}break;case 10:{node = document.createTextNode(s.substring(sp,sp+=u32buf[u32bufp++])); nodes[u32buf[u32bufp++]] = node; stack.push(node);}break;case 11:{stack.push(document.createElement(get_el()))}break;case 12:{stack.push(document.createElementNS(get_namespace(), get_el()))}break;case 13:{node = document.createElement('pre'); node.hidden = true; stack.push(node); nodes[u32buf[u32bufp++]] = node;}break;case 14:{node = document.createElement('pre'); node.hidden = true; stack.push(node);}break;case 15:event_name=get_evt();id=u32buf[u32bufp++];bubbles=u8buf[u8bufp++];
        bubbles = bubbles == 1;
        node = nodes[id];
        if(node.listening){
            node.listening += 1;
        } else {
            node.listening = 1;
        }
        node.setAttribute('data-dioxus-id', `${id}`);

        // if this is a mounted listener, we send the event immediately
        if (event_name === "mounted") {
            window.ipc.postMessage(
                serializeIpcMessage("user_event", {
                    name: event_name,
                    element: edit.id,
                    data: null,
                    bubbles,
                })
            );
        } else {
            listeners.create(event_name, node, bubbles, (event) => {
                handler(event, event_name, bubbles, config);
            });
        }break;case 16:{node = nodes[u32buf[u32bufp++]]; node.listening -= 1; node.removeAttribute('data-dioxus-id'); listeners.remove(node, get_evt(), u8buf[u8bufp++]);}break;case 17:{nodes[u32buf[u32bufp++]].textContent = s.substring(sp,sp+=u32buf[u32bufp++]);}break;case 18:{node = nodes[u32buf[u32bufp++]]; SetAttributeInner(node, get_attr(), s.substring(sp,sp+=u32buf[u32bufp++]), get_ns_cache());}break;case 19:{SetAttributeInner(stack[stack.length-1], get_attr(), s.substring(sp,sp+=u32buf[u32bufp++]), get_ns_cache());}break;case 20:id=u32buf[u32bufp++];field=get_attr();ns=get_ns_cache();{
            node = nodes[id];
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
        }break;case 21:{nodes[u32buf[u32bufp++]] = LoadChild((()=>{e=u8bufp+u32buf[u32bufp++];const final_array = u8buf.slice(u8bufp,e);u8bufp=e;return final_array;})());}break;case 22:array=(()=>{e=u8bufp+u32buf[u32bufp++];const final_array = u8buf.slice(u8bufp,e);u8bufp=e;return final_array;})();value=s.substring(sp,sp+=u32buf[u32bufp++]);id=u32buf[u32bufp++];{
            node = LoadChild(array);
            if (node.nodeType == Node.TEXT_NODE) {
                node.textContent = value;
            } else {
                let text = document.createTextNode(value);
                node.replaceWith(text);
                node = text;
            }
            nodes[id] = node;
        }break;case 23:{els = stack.splice(stack.length - u16buf[u16bufp++]); node = LoadChild((()=>{e=u8bufp+u32buf[u32bufp++];const final_array = u8buf.slice(u8bufp,e);u8bufp=e;return final_array;})()); node.replaceWith(...els);}break;case 24:{node = templates[u16buf[u16bufp++]][u16buf[u16bufp++]].cloneNode(true); nodes[u32buf[u32bufp++]] = node; stack.push(node);}break;case 25:{templates[u16buf[u16bufp++]] = stack.splice(stack.length-u16buf[u16bufp++]);}break;case 26:return true;
                        }
                        op>>>=8;
                    }
                }
            }
             function run_from_bytes(bytes){
                d = 0;
                update_memory(new Uint8Array(bytes))
                run()
            }
const config = new InterpreterConfig(false);