//! Convert a serialized event to an event Trigger
//!

use std::sync::Arc;
use std::{any::Any, rc::Rc};

use dioxus_core::{ElementId, EventPriority, ScopeId, UserEvent};
use dioxus_html::on::*;

#[derive(serde::Serialize, serde::Deserialize)]
struct ImEvent {
    event: String,
    mounted_dom_id: u64,
    scope: u64,
    contents: serde_json::Value,
}

pub fn trigger_from_serialized(val: serde_json::Value) -> UserEvent {
    let mut ims: Vec<ImEvent> = serde_json::from_value(val).unwrap();
    let ImEvent {
        event,
        mounted_dom_id,
        scope,
        contents,
    } = ims.into_iter().next().unwrap();

    let scope_id = ScopeId(scope as usize);
    let mounted_dom_id = Some(ElementId(mounted_dom_id as usize));

    let name = event_name_from_typ(&event);
    let event = make_synthetic_event(&event, contents);

    UserEvent {
        name,
        priority: EventPriority::Low,
        scope_id,
        mounted_dom_id,
        event,
    }
}

fn make_synthetic_event(name: &str, val: serde_json::Value) -> Arc<dyn Any + Send + Sync> {
    match name {
        "copy" | "cut" | "paste" => {
            //
            Arc::new(ClipboardEvent {})
        }
        "compositionend" | "compositionstart" | "compositionupdate" => {
            Arc::new(serde_json::from_value::<CompositionEvent>(val).unwrap())
        }
        "keydown" | "keypress" | "keyup" => {
            let evt = serde_json::from_value::<KeyboardEvent>(val).unwrap();
            Arc::new(evt)
        }
        "focus" | "blur" => {
            //
            Arc::new(FocusEvent {})
        }

        // todo: these handlers might get really slow if the input box gets large and allocation pressure is heavy
        // don't have a good solution with the serialized event problem
        "change" | "input" | "invalid" | "reset" | "submit" => {
            Arc::new(serde_json::from_value::<FormEvent>(val).unwrap())
        }

        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            Arc::new(serde_json::from_value::<MouseEvent>(val).unwrap())
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            Arc::new(serde_json::from_value::<PointerEvent>(val).unwrap())
        }
        "select" => {
            //
            Arc::new(serde_json::from_value::<SelectionEvent>(val).unwrap())
        }

        "touchcancel" | "touchend" | "touchmove" | "touchstart" => {
            Arc::new(serde_json::from_value::<TouchEvent>(val).unwrap())
        }

        "scroll" => Arc::new(()),

        "wheel" => Arc::new(serde_json::from_value::<WheelEvent>(val).unwrap()),

        "animationstart" | "animationend" | "animationiteration" => {
            Arc::new(serde_json::from_value::<AnimationEvent>(val).unwrap())
        }

        "transitionend" => Arc::new(serde_json::from_value::<TransitionEvent>(val).unwrap()),

        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => {
            //
            Arc::new(MediaEvent {})
        }

        "toggle" => Arc::new(ToggleEvent {}),

        _ => Arc::new(()),
    }
}

fn event_name_from_typ(typ: &str) -> &'static str {
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
