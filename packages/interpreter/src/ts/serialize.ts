// Handle serialization of the event data across the IPC boundarytype SerialziedEvent = {};

export type SerializedEvent = {
  values?: { [key: string]: FormDataEntryValue[] };
  value?: string;
  [key: string]: any;
};

export function serializeEvent(event: Event): SerializedEvent {
  if (event instanceof InputEvent) {
    if (event.target instanceof HTMLInputElement) {
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
    return {};
  }

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
    //     let files = [];
    //     if (event.dataTransfer && event.dataTransfer.files) {
    //       files = ["a", "b", "c"];
    //       // files = await serializeFileList(event.dataTransfer.files);
    //     }
    //     return { mouse: get_mouse_data(event), files };
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
