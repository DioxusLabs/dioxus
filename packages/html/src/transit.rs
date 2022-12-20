use std::{any::Any, rc::Rc};

use crate::events::*;
use dioxus_core::ElementId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct HtmlEvent {
    pub element: ElementId,
    pub name: String,
    pub bubbles: bool,
    pub data: EventData,
}

impl<'de> Deserialize<'de> for HtmlEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug, Clone)]
        struct Inner {
            element: ElementId,
            name: String,
            bubbles: bool,
            data: serde_value::Value,
        }

        let Inner {
            element,
            name,
            bubbles,
            data,
        } = Inner::deserialize(deserializer)?;

        Ok(HtmlEvent {
            data: fun_name(&name, data).unwrap(),
            element,
            bubbles,
            name,
        })
    }
}

fn fun_name(
    name: &str,
    data: serde_value::Value,
) -> Result<EventData, serde_value::DeserializerError> {
    use EventData::*;

    // a little macro-esque thing to make the code below more readable
    #[inline]
    fn de<'de, F>(f: serde_value::Value) -> Result<F, serde_value::DeserializerError>
    where
        F: Deserialize<'de>,
    {
        F::deserialize(f)
    }

    let data = match name {
        // Mouse
        "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => Mouse(de(data)?),

        // Clipboard
        "copy" | "cut" | "paste" => Clipboard(de(data)?),

        // Composition
        "compositionend" | "compositionstart" | "compositionupdate" => Composition(de(data)?),

        // Keyboard
        "keydown" | "keypress" | "keyup" => Keyboard(de(data)?),

        // Focus
        "blur" | "focus" | "focusin" | "focusout" => Focus(de(data)?),

        // Form
        "change" | "input" | "invalid" | "reset" | "submit" => Form(de(data)?),

        // Drag
        "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart"
        | "drop" => Drag(de(data)?),

        // Pointer
        "pointerlockchange" | "pointerlockerror" | "pointerdown" | "pointermove" | "pointerup"
        | "pointerover" | "pointerout" | "pointerenter" | "pointerleave" | "gotpointercapture"
        | "lostpointercapture" => Pointer(de(data)?),

        // Selection
        "selectstart" | "selectionchange" | "select" => Selection(de(data)?),

        // Touch
        "touchcancel" | "touchend" | "touchmove" | "touchstart" => Touch(de(data)?),

        // Srcoll
        "scroll" => Scroll(de(data)?),

        // Wheel
        "wheel" => Wheel(de(data)?),

        // Media
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "interruptbegin" | "interruptend" | "loadeddata" | "loadedmetadata"
        | "loadstart" | "pause" | "play" | "playing" | "progress" | "ratechange" | "seeked"
        | "seeking" | "stalled" | "suspend" | "timeupdate" | "volumechange" | "waiting"
        | "error" | "load" | "loadend" | "timeout" => Media(de(data)?),

        // Animation
        "animationstart" | "animationend" | "animationiteration" => Animation(de(data)?),

        // Transition
        "transitionend" => Transition(de(data)?),

        // Toggle
        "toggle" => Toggle(de(data)?),

        // ImageData => "load" | "error";
        // OtherData => "abort" | "afterprint" | "beforeprint" | "beforeunload" | "hashchange" | "languagechange" | "message" | "offline" | "online" | "pagehide" | "pageshow" | "popstate" | "rejectionhandled" | "storage" | "unhandledrejection" | "unload" | "userproximity" | "vrdisplayactivate" | "vrdisplayblur" | "vrdisplayconnect" | "vrdisplaydeactivate" | "vrdisplaydisconnect" | "vrdisplayfocus" | "vrdisplaypointerrestricted" | "vrdisplaypointerunrestricted" | "vrdisplaypresentchange";
        other => {
            return Err(serde_value::DeserializerError::UnknownVariant(
                other.to_string(),
                &[],
            ))
        }
    };

    Ok(data)
}

impl HtmlEvent {
    pub fn bubbles(&self) -> bool {
        event_bubbles(&self.name)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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
        bubbles: true,
    };

    println!("{}", serde_json::to_string_pretty(&data).unwrap());

    let o = r#"
{
  "element": 0,
  "name": "click",
  "bubbles": true,
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

    assert_eq!(data, p);
}
