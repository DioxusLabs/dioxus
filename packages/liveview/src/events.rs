#![allow(dead_code)]

//! Convert a serialized event to an event trigger

use std::any::Any;
use std::sync::Arc;

use dioxus_core::ElementId;
// use dioxus_html::event_bubbles;
use dioxus_html::events::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct IpcMessage {
    pub name: String,
    pub element: ElementId,
    pub bubbles: bool,
    pub data: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ImEvent {
    event: String,
    mounted_dom_id: ElementId,
    contents: serde_json::Value,
}

pub fn trigger_from_serialized(_val: serde_json::Value) {
    todo!()
    // let ImEvent {
    //     event,
    //     mounted_dom_id,
    //     contents,
    // } = serde_json::from_value(val).unwrap();

    // let mounted_dom_id = Some(mounted_dom_id);

    // let name = event_name_from_type(&event);
    // let event = make_synthetic_event(&event, contents);

    // UserEvent {
    //     name,
    //     scope_id: None,
    //     element: mounted_dom_id,
    //     data: event,
    //     bubbles: event_bubbles(name),
    // }
}

fn make_synthetic_event(name: &str, val: serde_json::Value) -> Arc<dyn Any> {
    match name {
        "copy" | "cut" | "paste" => {
            //
            Arc::new(ClipboardData {})
        }
        "compositionend" | "compositionstart" | "compositionupdate" => {
            Arc::new(serde_json::from_value::<CompositionData>(val).unwrap())
        }
        "keydown" | "keypress" | "keyup" => {
            let evt = serde_json::from_value::<KeyboardData>(val).unwrap();
            Arc::new(evt)
        }
        "focus" | "blur" | "focusout" | "focusin" => {
            //
            Arc::new(FocusData {})
        }

        // todo: these handlers might get really slow if the input box gets large and allocation pressure is heavy
        // don't have a good solution with the serialized event problem
        "change" | "input" | "invalid" | "reset" | "submit" => {
            Arc::new(serde_json::from_value::<FormData>(val).unwrap())
        }

        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            Arc::new(serde_json::from_value::<MouseData>(val).unwrap())
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            Arc::new(serde_json::from_value::<PointerData>(val).unwrap())
        }
        "select" => {
            //
            Arc::new(serde_json::from_value::<SelectionData>(val).unwrap())
        }

        "touchcancel" | "touchend" | "touchmove" | "touchstart" => {
            Arc::new(serde_json::from_value::<TouchData>(val).unwrap())
        }

        "scroll" => Arc::new(()),

        "wheel" => Arc::new(serde_json::from_value::<WheelData>(val).unwrap()),

        "animationstart" | "animationend" | "animationiteration" => {
            Arc::new(serde_json::from_value::<AnimationData>(val).unwrap())
        }

        "transitionend" => Arc::new(serde_json::from_value::<TransitionData>(val).unwrap()),

        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => {
            //
            Arc::new(MediaData {})
        }

        "toggle" => Arc::new(ToggleData {}),

        _ => Arc::new(()),
    }
}

fn event_name_from_type(typ: &str) -> &'static str {
    match typ {
        "copy" => "copy",
        "cut" => "cut",
        "paste" => "paste",
        "compositionend" => "compositionend",
        "compositionstart" => "compositionstart",
        "compositionupdate" => "compositionupdate",
        "keydown" => "keydown",
        "keypress" => "keypress",
        "keyup" => "keyup",
        "focus" => "focus",
        "focusout" => "focusout",
        "focusin" => "focusin",
        "blur" => "blur",
        "change" => "change",
        "input" => "input",
        "invalid" => "invalid",
        "reset" => "reset",
        "submit" => "submit",
        "click" => "click",
        "contextmenu" => "contextmenu",
        "doubleclick" => "doubleclick",
        "drag" => "drag",
        "dragend" => "dragend",
        "dragenter" => "dragenter",
        "dragexit" => "dragexit",
        "dragleave" => "dragleave",
        "dragover" => "dragover",
        "dragstart" => "dragstart",
        "drop" => "drop",
        "mousedown" => "mousedown",
        "mouseenter" => "mouseenter",
        "mouseleave" => "mouseleave",
        "mousemove" => "mousemove",
        "mouseout" => "mouseout",
        "mouseover" => "mouseover",
        "mouseup" => "mouseup",
        "pointerdown" => "pointerdown",
        "pointermove" => "pointermove",
        "pointerup" => "pointerup",
        "pointercancel" => "pointercancel",
        "gotpointercapture" => "gotpointercapture",
        "lostpointercapture" => "lostpointercapture",
        "pointerenter" => "pointerenter",
        "pointerleave" => "pointerleave",
        "pointerover" => "pointerover",
        "pointerout" => "pointerout",
        "select" => "select",
        "touchcancel" => "touchcancel",
        "touchend" => "touchend",
        "touchmove" => "touchmove",
        "touchstart" => "touchstart",
        "scroll" => "scroll",
        "wheel" => "wheel",
        "animationstart" => "animationstart",
        "animationend" => "animationend",
        "animationiteration" => "animationiteration",
        "transitionend" => "transitionend",
        "abort" => "abort",
        "canplay" => "canplay",
        "canplaythrough" => "canplaythrough",
        "durationchange" => "durationchange",
        "emptied" => "emptied",
        "encrypted" => "encrypted",
        "ended" => "ended",
        "error" => "error",
        "loadeddata" => "loadeddata",
        "loadedmetadata" => "loadedmetadata",
        "loadstart" => "loadstart",
        "pause" => "pause",
        "play" => "play",
        "playing" => "playing",
        "progress" => "progress",
        "ratechange" => "ratechange",
        "seeked" => "seeked",
        "seeking" => "seeking",
        "stalled" => "stalled",
        "suspend" => "suspend",
        "timeupdate" => "timeupdate",
        "volumechange" => "volumechange",
        "waiting" => "waiting",
        "toggle" => "toggle",
        _ => {
            panic!("unsupported event type")
        }
    }
}
