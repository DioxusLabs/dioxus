//! Implementation of a renderer for Dioxus on the web.
//!
//! Oustanding todos:
//! - Removing event listeners (delegation)
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?>

use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::{event_bubbles, CompositionData, FormData};
use dioxus_interpreter_js::Interpreter;
use futures_channel::mpsc;
use js_sys::Function;
use std::{any::Any, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Document, Element, Event, HtmlElement};

use crate::Config;

pub struct WebsysDom {
    pub interpreter: Interpreter,

    pub(crate) root: Element,

    pub handler: Closure<dyn FnMut(&Event)>,
}

impl WebsysDom {
    pub fn new(cfg: Config, event_channel: mpsc::UnboundedSender<Event>) -> Self {
        // eventually, we just want to let the interpreter do all the work of decoding events into our event type
        let callback: Box<dyn FnMut(&Event)> = Box::new(move |event: &web_sys::Event| {
            _ = event_channel.unbounded_send(event.clone());

            // if let Ok(synthetic_event) = decoded {
            //     // Try to prevent default if the attribute is set
            //     if let Some(node) = target.dyn_ref::<HtmlElement>() {
            //         if let Some(name) = node.get_attribute("dioxus-prevent-default") {
            //             if name == synthetic_event.name
            //                 || name.trim_start_matches("on") == synthetic_event.name
            //             {
            //                 log::trace!("Preventing default");
            //                 event.prevent_default();
            //             }
            //         }
            //     }

            //     sender_callback.as_ref()(SchedulerMsg::Event(synthetic_event))
            // }
        });

        // a match here in order to avoid some error during runtime browser test
        let document = load_document();
        let root = match document.get_element_by_id(&cfg.rootname) {
            Some(root) => root,
            None => document.create_element("body").ok().unwrap(),
        };

        Self {
            interpreter: Interpreter::new(root.clone()),
            handler: Closure::wrap(callback),
            root,
        }
    }

    pub fn apply_edits(&mut self, mut edits: Vec<Mutation>) {
        use Mutation::*;
        let i = &self.interpreter;

        for edit in edits.drain(..) {
            match edit {
                AppendChildren { m } => i.AppendChildren(m as u32),
                AssignId { path, id } => i.AssignId(path, id.0 as u32),
                CreateElement { name } => i.CreateElement(name),
                CreateElementNamespace { name, namespace } => i.CreateElementNs(name, namespace),
                CreatePlaceholder { id } => i.CreatePlaceholder(id.0 as u32),
                CreateStaticPlaceholder => i.CreateStaticPlaceholder(),
                CreateTextPlaceholder => i.CreateTextPlaceholder(),
                CreateStaticText { value } => i.CreateStaticText(value),
                CreateTextNode { value, id } => i.CreateTextNode(value.into(), id.0 as u32),
                HydrateText { path, value, id } => i.HydrateText(path, value, id.0 as u32),
                LoadTemplate { name, index, id } => i.LoadTemplate(name, index as u32, id.0 as u32),
                ReplaceWith { id, m } => i.ReplaceWith(id.0 as u32, m as u32),
                ReplacePlaceholder { path, m } => i.ReplacePlaceholder(path, m as u32),
                InsertAfter { id, m } => i.InsertAfter(id.0 as u32, m as u32),
                InsertBefore { id, m } => i.InsertBefore(id.0 as u32, m as u32),
                SaveTemplate { name, m } => i.SaveTemplate(name, m as u32),
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => i.SetAttribute(id.0 as u32, name, value.into(), ns),
                SetStaticAttribute { name, value, ns } => {
                    i.SetStaticAttribute(name, value.into(), ns)
                }
                SetBoolAttribute { name, value, id } => {
                    i.SetBoolAttribute(id.0 as u32, name, value)
                }
                SetText { value, id } => i.SetText(id.0 as u32, value.into()),
                NewEventListener { name, scope, id } => {
                    let handler: &Function = self.handler.as_ref().unchecked_ref();
                    self.interpreter.NewEventListener(
                        name,
                        id.0 as u32,
                        handler,
                        event_bubbles(&name[2..]),
                    );
                }
                RemoveEventListener { name, id } => i.RemoveEventListener(name, id.0 as u32),
                Remove { id } => i.Remove(id.0 as u32),
                PushRoot { id } => i.PushRoot(id.0 as u32),
                // Mutation::RemoveEventListener { root, name: event } => self
                //     .interpreter
                //     .RemoveEventListener(root, event, event_bubbles(event)),
            }
        }
    }
}

// todo: some of these events are being casted to the wrong event type.
// We need tests that simulate clicks/etc and make sure every event type works.
pub fn virtual_event_from_websys_event(event: web_sys::Event, target: Element) -> Rc<dyn Any> {
    use dioxus_html::events::*;

    match event.type_().as_str() {
        "copy" | "cut" | "paste" => Rc::new(ClipboardData {}),
        "compositionend" | "compositionstart" | "compositionupdate" => {
            make_composition_event(&event)
        }
        "keydown" | "keypress" | "keyup" => Rc::new(KeyboardData::from(event)),
        "focus" | "blur" | "focusout" | "focusin" => Rc::new(FocusData {}),

        "change" | "input" | "invalid" | "reset" | "submit" => read_input_to_data(target),

        "click" | "contextmenu" | "dblclick" | "doubleclick" | "drag" | "dragend" | "dragenter"
        | "dragexit" | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown"
        | "mouseenter" | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            Rc::new(MouseData::from(event))
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            Rc::new(PointerData::from(event))
        }
        "select" => Rc::new(SelectionData {}),
        "touchcancel" | "touchend" | "touchmove" | "touchstart" => Rc::new(TouchData::from(event)),

        "scroll" => Rc::new(()),
        "wheel" => Rc::new(WheelData::from(event)),
        "animationstart" | "animationend" | "animationiteration" => {
            Rc::new(AnimationData::from(event))
        }
        "transitionend" => Rc::new(TransitionData::from(event)),
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => Rc::new(MediaData {}),
        "toggle" => Rc::new(ToggleData {}),

        _ => Rc::new(()),
    }
}

fn make_composition_event(event: &Event) -> Rc<CompositionData> {
    let evt: &web_sys::CompositionEvent = event.dyn_ref().unwrap();
    Rc::new(CompositionData {
        data: evt.data().unwrap_or_default(),
    })
}

pub(crate) fn load_document() -> Document {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
}

fn read_input_to_data(target: Element) -> Rc<FormData> {
    // todo: these handlers might get really slow if the input box gets large and allocation pressure is heavy
    // don't have a good solution with the serialized event problem

    let value: String = target
        .dyn_ref()
        .map(|input: &web_sys::HtmlInputElement| {
            // todo: special case more input types
            match input.type_().as_str() {
                "checkbox" => {
                    match input.checked() {
                        true => "true".to_string(),
                        false => "false".to_string(),
                    }
                },
                _ => {
                    input.value()
                }
            }
        })
        .or_else(|| {
            target
                .dyn_ref()
                .map(|input: &web_sys::HtmlTextAreaElement| input.value())
        })
        // select elements are NOT input events - because - why woudn't they be??
        .or_else(|| {
            target
                .dyn_ref()
                .map(|input: &web_sys::HtmlSelectElement| input.value())
        })
        .or_else(|| {
            target
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .text_content()
        })
        .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener");

    let mut values = std::collections::HashMap::new();

    // try to fill in form values
    if let Some(form) = target.dyn_ref::<web_sys::HtmlFormElement>() {
        let elements = form.elements();
        for x in 0..elements.length() {
            let element = elements.item(x).unwrap();
            if let Some(name) = element.get_attribute("name") {
                let value: Option<String> = element
                    .dyn_ref()
                    .map(|input: &web_sys::HtmlInputElement| {
                        match input.type_().as_str() {
                            "checkbox" => {
                                match input.checked() {
                                    true => Some("true".to_string()),
                                    false => Some("false".to_string()),
                                }
                            },
                            "radio" => {
                                match input.checked() {
                                    true => Some(input.value()),
                                    false => None,
                                }
                            }
                            _ => Some(input.value())
                        }
                    })
                    .or_else(|| element.dyn_ref().map(|input: &web_sys::HtmlTextAreaElement| Some(input.value())))
                    .or_else(|| element.dyn_ref().map(|input: &web_sys::HtmlSelectElement| Some(input.value())))
                    .or_else(|| Some(element.dyn_ref::<web_sys::HtmlElement>().unwrap().text_content()))
                    .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener");
                if let Some(value) = value {
                    values.insert(name, value);
                }
            }
        }
    }

    Rc::new(FormData {
        value,
        values,
        files: None,
    })
}
