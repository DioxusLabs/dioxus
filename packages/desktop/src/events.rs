//! Convert a serialized event to an event trigger

use dioxus_html::events::*;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use std::any::Any;
use std::rc::Rc;

#[derive(Deserialize, Serialize)]
pub(crate) struct IpcMessage {
    method: String,
    params: serde_json::Value,
}

impl IpcMessage {
    pub(crate) fn method(&self) -> &str {
        self.method.as_str()
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}

pub(crate) fn parse_ipc_message(payload: &str) -> Option<IpcMessage> {
    match serde_json::from_str(payload) {
        Ok(message) => Some(message),
        Err(e) => {
            log::error!("could not parse IPC message, error: {}", e);
            None
        }
    }
}

macro_rules! match_data {
    (
        $m:ident;
        $name:ident;
        $(
            $tip:ty => $($mname:literal)|* ;
        )*
    ) => {
        match $name {
            $( $($mname)|* => {
                println!("casting to type {:?}", std::any::TypeId::of::<$tip>());
                let val: $tip = from_value::<$tip>($m).ok()?;
                Rc::new(val) as Rc<dyn Any>
            })*
            _ => return None,
        }
    };
}

#[derive(Deserialize)]
pub struct EventMessage {
    pub contents: serde_json::Value,
    pub event: String,
    pub mounted_dom_id: usize,
}

pub fn decode_event(value: EventMessage) -> Option<Rc<dyn Any>> {
    let val = value.contents;
    let name = value.event.as_str();
    type DragData = MouseData;

    let evt = match_data! { val; name;
        MouseData => "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter" | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup";
        ClipboardData => "copy" | "cut" | "paste";
        CompositionData => "compositionend" | "compositionstart" | "compositionupdate";
        KeyboardData => "keydown" | "keypress" | "keyup";
        FocusData => "blur" | "focus" | "focusin" | "focusout";
        FormData => "change" | "input" | "invalid" | "reset" | "submit";
        DragData => "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart" | "drop";
        PointerData => "pointerlockchange" | "pointerlockerror" | "pointerdown" | "pointermove" | "pointerup" | "pointerover" | "pointerout" | "pointerenter" | "pointerleave" | "gotpointercapture" | "lostpointercapture";
        SelectionData => "selectstart" | "selectionchange" | "select";
        TouchData => "touchcancel" | "touchend" | "touchmove" | "touchstart";
        ScrollData => "scroll";
        WheelData => "wheel";
        MediaData => "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied"
            | "encrypted" | "ended" | "interruptbegin" | "interruptend" | "loadeddata"
            | "loadedmetadata" | "loadstart" | "pause" | "play" | "playing" | "progress"
            | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend" | "timeupdate"
            | "volumechange" | "waiting" | "error" | "load" | "loadend" | "timeout";
        AnimationData => "animationstart" | "animationend" | "animationiteration";
        TransitionData => "transitionend";
        ToggleData => "toggle";
        // ImageData => "load" | "error";
        // OtherData => "abort" | "afterprint" | "beforeprint" | "beforeunload" | "hashchange" | "languagechange" | "message" | "offline" | "online" | "pagehide" | "pageshow" | "popstate" | "rejectionhandled" | "storage" | "unhandledrejection" | "unload" | "userproximity" | "vrdisplayactivate" | "vrdisplayblur" | "vrdisplayconnect" | "vrdisplaydeactivate" | "vrdisplaydisconnect" | "vrdisplayfocus" | "vrdisplaypointerrestricted" | "vrdisplaypointerunrestricted" | "vrdisplaypresentchange";
    };

    Some(evt)
}
