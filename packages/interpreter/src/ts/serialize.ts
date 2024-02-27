// Handle serialization of the event data across the IPC boundarytype SerialziedEvent = {};

type SerializedEvent = {
  values?: { [key: string]: FormDataEntryValue[] } | FormDataEntryValue[];
  value?: string;
  [key: string]: any;
};

function serializeEvent(event: Event): SerializedEvent {

  // copy, cut, paste
  if (event instanceof KeyboardEvent) {
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
  }


  // "pointerdown" "pointermove" "pointerup" "pointercancel" "gotpointercapture" "lostpointercapture" "pointerenter"
  // "pointerleave" "pointerover" "pointerout"
  if (event instanceof PointerEvent) {
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
  }

  if (event instanceof MouseEvent) {
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
  }

  if (event instanceof TouchEvent) {
    return {
      alt_key: event.altKey,
      ctrl_key: event.ctrlKey,
      meta_key: event.metaKey,
      shift_key: event.shiftKey,
      changed_touches: event.changedTouches,
      target_touches: event.targetTouches,
      touches: event.touches,
    };
  }

  if (event instanceof WheelEvent) {
    return {
      delta_x: event.deltaX,
      delta_y: event.deltaY,
      delta_z: event.deltaZ,
      delta_mode: event.deltaMode,
    };
  }

  if (event instanceof AnimationEvent) {
    return {
      animation_name: event.animationName,
      elapsed_time: event.elapsedTime,
      pseudo_element: event.pseudoElement,
    };
  }

  if (event instanceof TransitionEvent) {
    return {
      property_name: event.propertyName,
      elapsed_time: event.elapsedTime,
      pseudo_element: event.pseudoElement,
    };
  }

  if (event instanceof ClipboardEvent) {
    return {};
  }

  if (event instanceof CompositionEvent) {
    return {
      data: event.data,
    };
  }

  if (event instanceof DragEvent) {
    return {
      mouse: {
        alt_key: event.altKey,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
        shift_key: event.shiftKey,
      },
      files: [],
    };
  }

  if (event instanceof FocusEvent) {
    return {};
  }

  return {};
}

function isElementNode(node: Node) {
  return node.nodeType == 1;
}

function eventBubbles(eventName: string) {
  switch (eventName) {
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



// switch (event.type) {
//   case "copy": case "cut": case "paste": {
//     return {};
//   }
//   case "compositionend":
//   case "compositionstart":
//   case "compositionupdate": {
//     let { data } = event;
//     return {
//       data,
//     };
//   }
//   case "keydown": case "keypress": case "keyup": {
//     if (event instanceof KeyboardEvent) {
//       return {
//         char_code: event.charCode,
//         is_composing: event.isComposing,
//         key: event.key,
//         alt_key: event.altKey,
//         ctrl_key: event.ctrlKey,
//         meta_key: event.metaKey,
//         key_code: event.keyCode,
//         shift_key: event.shiftKey,
//         location: event.location,
//         repeat: event.repeat,
//         which: event.which,
//         code: event.code,
//       };
//     };
//   }
//   case "focus":
//   case "blur": {
//     return {};
//   }
//   case "change": {
//     let target = event.target;
//     let value;
//     if (target.type === "checkbox" || target.type === "radio") {
//       value = target.checked ? "true" : "false";
//     } else {
//       value = target.value ?? target.textContent;
//     }
//     return {
//       value: value,
//       values: {},
//     };
//   }
//   case "input":
//   case "invalid":
//   case "reset":
//   case "submit": {
//     let target = event.target;
//     let value = target.value ?? target.textContent;

//     if (target.type === "checkbox") {
//       value = target.checked ? "true" : "false";
//     }

//     return {
//       value: value,
//       values: {},
//     };
//   }
//   case "drag":
//   case "dragend":
//   case "dragenter":
//   case "dragexit":
//   case "dragleave":
//   case "dragover":
//   case "dragstart":
//   case "drop": {
//     let files = [];

//     if (event.dataTransfer && event.dataTransfer.files) {
//       files = ["a", "b", "c"];
//       // files = await serializeFileList(event.dataTransfer.files);
//     }

//     return { mouse: get_mouse_data(event), files };
//   }
//   case "click":
//   case "contextmenu":
//   case "doubleclick":
//   case "dblclick":
//   case "mousedown":
//   case "mouseenter":
//   case "mouseleave":
//   case "mousemove":
//   case "mouseout":
//   case "mouseover":
//   case "mouseup": {
//     return get_mouse_data(event);
//   }
//   case "select": {
//     return {};
//   }
//   case "touchcancel":
//   case "touchend":
//   case "touchmove":
//   case "touchstart": {
//     const { altKey, ctrlKey, metaKey, shiftKey } = event;
//     return {
//       // changed_touches: event.changedTouches,
//       // target_touches: event.targetTouches,
//       // touches: event.touches,
//       alt_key: altKey,
//       ctrl_key: ctrlKey,
//       meta_key: metaKey,
//       shift_key: shiftKey,
//     };
//   }
//   case "scroll": {
//     return {};
//   }
//   case "wheel": {
//     const { deltaX, deltaY, deltaZ, deltaMode } = event;
//     return {
//       delta_x: deltaX,
//       delta_y: deltaY,
//       delta_z: deltaZ,
//       delta_mode: deltaMode,
//     };
//   }
//   case "animationstart":
//   case "animationend":
//   case "animationiteration": {
//     const { animationName, elapsedTime, pseudoElement } = event;
//     return {
//       animation_name: animationName,
//       elapsed_time: elapsedTime,
//       pseudo_element: pseudoElement,
//     };
//   }
//   case "transitionend": {
//     const { propertyName, elapsedTime, pseudoElement } = event;
//     return {
//       property_name: propertyName,
//       elapsed_time: elapsedTime,
//       pseudo_element: pseudoElement,
//     };
//   }
//   case "abort":
//   case "canplay":
//   case "canplaythrough":
//   case "durationchange":
//   case "emptied":
//   case "encrypted":
//   case "ended":
//   case "error":
//   case "loadeddata":
//   case "loadedmetadata":
//   case "loadstart":
//   case "pause":
//   case "play":
//   case "playing":
//   case "progress":
//   case "ratechange":
//   case "seeked":
//   case "seeking":
//   case "stalled":
//   case "suspend":
//   case "timeupdate":
//   case "volumechange":
//   case "waiting": {
//     return {};
//   }
//   case "toggle": {
//     return {};
//   }
//   default: {
//     return {};
//   }
// }
