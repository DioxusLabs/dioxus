use std::{any::Any, rc::Rc};

use crate::events::*;
use dioxus_core::ElementId;
use serde::{Deserialize, Serialize};

// macro_rules! match_data {
//     (
//         $m:ident;
//         $name:ident;
//         $(
//             $tip:ty => $($mname:literal)|* ;
//         )*
//     ) => {
//         match $name {
//             $( $($mname)|* => {
//                 let val: $tip = from_value::<$tip>($m).ok()?;
//                 Rc::new(val) as Rc<dyn Any>
//             })*
//             _ => return None,
//         }
//     };
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HtmlEvent {
    pub element: ElementId,
    pub name: String,
    pub data: EventData,
}

impl HtmlEvent {
    pub fn bubbles(&self) -> bool {
        event_bubbles(&self.name)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum EventData {
    Mouse(MouseData),
    Clipboard(ClipboardData),
    Composition(CompositionData),
    Keyboard(KeyboardData),
    Focus(FocusData),
    Form(FormData),
    Drag(DragData),
    Pointer(PointerData),
    Selection(SelectionData),
    Touch(TouchData),
    Scroll(ScrollData),
    Wheel(WheelData),
    Media(MediaData),
    Animation(AnimationData),
    Transition(TransitionData),
    Toggle(ToggleData),
}

impl EventData {
    pub fn into_any(self) -> Rc<dyn Any> {
        match self {
            EventData::Mouse(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Clipboard(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Composition(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Keyboard(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Focus(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Form(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Drag(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Pointer(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Selection(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Touch(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Scroll(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Wheel(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Media(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Animation(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Transition(data) => Rc::new(data) as Rc<dyn Any>,
            EventData::Toggle(data) => Rc::new(data) as Rc<dyn Any>,
        }
    }
}

#[test]
fn test_back_and_forth() {
    let data = HtmlEvent {
        element: ElementId(0),
        data: EventData::Mouse(MouseData::default()),
        name: "click".to_string(),
    };

    println!("{}", serde_json::to_string_pretty(&data).unwrap());

    let o = r#"
{
  "element": 0,
  "name": "click",
  "data": {
    "alt_key": false,
    "button": 0,
    "buttons": 0,
    "client_x": 0,
    "client_y": 0,
    "ctrl_key": false,
    "meta_key": false,
    "offset_x": 0,
    "offset_y": 0,
    "page_x": 0,
    "page_y": 0,
    "screen_x": 0,
    "screen_y": 0,
    "shift_key": false
  }
}
    "#;

    let p: HtmlEvent = serde_json::from_str(o).unwrap();
}

// pub fn decode_event(value: ) -> Option<Rc<dyn Any>> {
//     let val = value.data;
//     let name = value.event.as_str();
//     type DragData = MouseData;

//     let evt = match_data! { val; name;
//         MouseData => "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter" | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup";
//         ClipboardData => "copy" | "cut" | "paste";
//         CompositionData => "compositionend" | "compositionstart" | "compositionupdate";
//         KeyboardData => "keydown" | "keypress" | "keyup";
//         FocusData => "blur" | "focus" | "focusin" | "focusout";
//         FormData => "change" | "input" | "invalid" | "reset" | "submit";
//         DragData => "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart" | "drop";
//         PointerData => "pointerlockchange" | "pointerlockerror" | "pointerdown" | "pointermove" | "pointerup" | "pointerover" | "pointerout" | "pointerenter" | "pointerleave" | "gotpointercapture" | "lostpointercapture";
//         SelectionData => "selectstart" | "selectionchange" | "select";
//         TouchData => "touchcancel" | "touchend" | "touchmove" | "touchstart";
//         ScrollData => "scroll";
//         WheelData => "wheel";
//         MediaData => "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied"
//             | "encrypted" | "ended" | "interruptbegin" | "interruptend" | "loadeddata"
//             | "loadedmetadata" | "loadstart" | "pause" | "play" | "playing" | "progress"
//             | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend" | "timeupdate"
//             | "volumechange" | "waiting" | "error" | "load" | "loadend" | "timeout";
//         AnimationData => "animationstart" | "animationend" | "animationiteration";
//         TransitionData => "transitionend";
//         ToggleData => "toggle";
//         // ImageData => "load" | "error";
//         // OtherData => "abort" | "afterprint" | "beforeprint" | "beforeunload" | "hashchange" | "languagechange" | "message" | "offline" | "online" | "pagehide" | "pageshow" | "popstate" | "rejectionhandled" | "storage" | "unhandledrejection" | "unload" | "userproximity" | "vrdisplayactivate" | "vrdisplayblur" | "vrdisplayconnect" | "vrdisplaydeactivate" | "vrdisplaydisconnect" | "vrdisplayfocus" | "vrdisplaypointerrestricted" | "vrdisplaypointerunrestricted" | "vrdisplaypresentchange";
//     };

//     Some(evt)
// }
