// Handle serialization of the event data across the IPC boundarytype SerializedEvent = {};

import { retrieveSelectValue, retrieveValues } from "./form";

export type AppTouchEvent = TouchEvent;

export type SerializedEvent = {
  values?: { [key: string]: FormDataEntryValue[] };
  value?: string;
  [key: string]: any;
};

export function serializeEvent(
  event: Event,
  target: EventTarget
): SerializedEvent {
  let contents = {};

  // merge the object into the contents
  let extend = (obj: any) => (contents = { ...contents, ...obj });

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
    extend({
      property_name: event.propertyName,
      elapsed_time: event.elapsedTime,
      pseudo_element: event.pseudoElement,
    });
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

  // safari is quirky and doesn't have TouchEvent
  if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent) {
    extend(serializeTouchEvent(event));
  }

  if (
    event.type === "submit" ||
    event.type === "reset" ||
    event.type === "click" ||
    event.type === "change" ||
    event.type === "input"
  ) {
    extend(serializeInputEvent(event as InputEvent, target));
  }

  // If there's any files, we need to serialize them
  if (event instanceof DragEvent) {
    // let files: { [key: string]: Uint8Array } = {};
    // if (event.dataTransfer && event.dataTransfer.files) {
    //   files["a"] = new Uint8Array(0);
    //   // files = {
    //   //   entries: Array.from(event.dataTransfer.files).map((file) => {
    //   //     return {
    //   //       name: file.name,
    //   //       type: file.type,
    //   //       size: file.size,
    //   //       last_modified: file.lastModified,
    //   //     };
    //   //   }
    //   // };
    //   // files = await serializeFileList(event.dataTransfer.files);
    // }
    // extend({ files: files });
  }

  return contents;
}

function serializeInputEvent(
  event: InputEvent,
  target: EventTarget
): SerializedEvent {
  let contents: SerializedEvent = {};

  // Attempt to retrieve the values from the form
  if (target instanceof HTMLElement) {
    let values = retrieveValues(event, target);
    contents.values = values.values;
    contents.valid = values.valid;
  }

  if (event.target instanceof HTMLInputElement) {
    let target = event.target;
    let value = target.value ?? target.textContent ?? "";

    if (target.type === "checkbox") {
      value = target.checked ? "true" : "false";
    } else if (target.type === "radio") {
      value = target.value;
    }

    contents.value = value;
  }

  if (event.target instanceof HTMLTextAreaElement) {
    contents.value = event.target.value;
  }

  if (event.target instanceof HTMLSelectElement) {
    contents.value = retrieveSelectValue(event.target).join(",");
  }

  // Ensure the serializer isn't quirky
  if (contents.value === undefined) {
    contents.value = "";
  }

  return contents;
}

function serializeWheelEvent(event: WheelEvent): SerializedEvent {
  return {
    delta_x: event.deltaX,
    delta_y: event.deltaY,
    delta_z: event.deltaZ,
    delta_mode: event.deltaMode,
  };
}

function serializeTouchEvent(event: TouchEvent): SerializedEvent {
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

function serializePointerEvent(event: PointerEvent): SerializedEvent {
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

function serializeMouseEvent(event: MouseEvent): SerializedEvent {
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

function serializeKeyboardEvent(event: KeyboardEvent): SerializedEvent {
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

function serializeAnimationEvent(event: AnimationEvent): SerializedEvent {
  return {
    animation_name: event.animationName,
    elapsed_time: event.elapsedTime,
    pseudo_element: event.pseudoElement,
  };
}

function serializeDragEvent(event: DragEvent): SerializedEvent {
  let files = undefined;
  // On desktop if there is file data, we insert it from wry. We just add a placeholder to let the rust side of dioxus know there's files
  if (
    event.dataTransfer &&
    event.dataTransfer.files &&
    event.dataTransfer.files.length > 0
  ) {
    files = {
      files: { placeholder: [] },
    };
  }
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
}
