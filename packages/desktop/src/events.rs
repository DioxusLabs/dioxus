//! Convert a serialized event to an event Trigger
//!

use std::rc::Rc;
use std::sync::Arc;

use dioxus_core::{
    events::{
        on::{MouseEvent, MouseEventInner},
        SyntheticEvent,
    },
    ElementId, EventPriority, ScopeId, UserEvent,
};

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

    let scope = ScopeId(scope as usize);
    let mounted_dom_id = Some(ElementId(mounted_dom_id as usize));

    let name = event_name_from_typ(&event);
    let event = make_synthetic_event(&event, contents);

    UserEvent {
        name,
        event,
        scope,
        mounted_dom_id,
    }
}

fn make_synthetic_event(name: &str, val: serde_json::Value) -> SyntheticEvent {
    use dioxus_core::events::on::*;
    use dioxus_core::events::DioxusEvent;

    match name {
        "copy" | "cut" | "paste" => SyntheticEvent::ClipboardEvent(ClipboardEvent(
            DioxusEvent::new(ClipboardEventInner(), ()),
        )),
        "compositionend" | "compositionstart" | "compositionupdate" => {
            SyntheticEvent::CompositionEvent(CompositionEvent(DioxusEvent::new(
                serde_json::from_value(val).unwrap(),
                (),
            )))
        }
        "keydown" | "keypress" | "keyup" => SyntheticEvent::KeyboardEvent(KeyboardEvent(
            DioxusEvent::new(serde_json::from_value(val).unwrap(), ()),
        )),
        "focus" | "blur" => {
            SyntheticEvent::FocusEvent(FocusEvent(DioxusEvent::new(FocusEventInner {}, ())))
        }
        "change" => SyntheticEvent::GenericEvent(DioxusEvent::new((), ())),

        // todo: these handlers might get really slow if the input box gets large and allocation pressure is heavy
        // don't have a good solution with the serialized event problem
        "input" | "invalid" | "reset" | "submit" => SyntheticEvent::FormEvent(FormEvent(
            DioxusEvent::new(serde_json::from_value(val).unwrap(), ()),
        )),
        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            SyntheticEvent::MouseEvent(MouseEvent(DioxusEvent::new(
                serde_json::from_value(val).unwrap(),
                (),
            )))
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            SyntheticEvent::PointerEvent(PointerEvent(DioxusEvent::new(
                serde_json::from_value(val).unwrap(),
                (),
            )))
        }
        "select" => SyntheticEvent::SelectionEvent(SelectionEvent(DioxusEvent::new(
            SelectionEventInner {},
            (),
        ))),

        "touchcancel" | "touchend" | "touchmove" | "touchstart" => SyntheticEvent::TouchEvent(
            TouchEvent(DioxusEvent::new(serde_json::from_value(val).unwrap(), ())),
        ),

        "scroll" => SyntheticEvent::GenericEvent(DioxusEvent::new((), ())),

        "wheel" => SyntheticEvent::WheelEvent(WheelEvent(DioxusEvent::new(
            serde_json::from_value(val).unwrap(),
            (),
        ))),

        "animationstart" | "animationend" | "animationiteration" => SyntheticEvent::AnimationEvent(
            AnimationEvent(DioxusEvent::new(serde_json::from_value(val).unwrap(), ())),
        ),

        "transitionend" => SyntheticEvent::TransitionEvent(TransitionEvent(DioxusEvent::new(
            serde_json::from_value(val).unwrap(),
            (),
        ))),

        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => {
            SyntheticEvent::MediaEvent(MediaEvent(DioxusEvent::new(MediaEventInner {}, ())))
        }

        "toggle" => {
            SyntheticEvent::ToggleEvent(ToggleEvent(DioxusEvent::new(ToggleEventInner {}, ())))
        }

        _ => SyntheticEvent::GenericEvent(DioxusEvent::new((), ())),
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
