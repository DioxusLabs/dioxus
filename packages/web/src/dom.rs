//! Implementation of a renderer for Dioxus on the web.
//!
//! Oustanding todos:
//! - Removing event listeners (delegation)
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?>

use dioxus_core::{DomEdit, ElementId, SchedulerMsg, UserEvent};
use dioxus_interpreter_js::Interpreter;
use js_sys::Function;
use std::{any::Any, rc::Rc, sync::Arc};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

use crate::WebConfig;

pub struct WebsysDom {
    pub interpreter: Interpreter,

    pub(crate) root: Element,

    pub handler: Closure<dyn FnMut(&Event)>,
}

impl WebsysDom {
    pub fn new(cfg: WebConfig, sender_callback: Rc<dyn Fn(SchedulerMsg)>) -> Self {
        // eventually, we just want to let the interpreter do all the work of decoding events into our event type
        let callback: Box<dyn FnMut(&Event)> = Box::new(move |event: &web_sys::Event| {
            let mut target = event
                .target()
                .expect("missing target")
                .dyn_into::<Element>()
                .expect("not a valid element");

            let typ = event.type_();

            let decoded: anyhow::Result<UserEvent> = loop {
                match target.get_attribute("data-dioxus-id").map(|f| f.parse()) {
                    Some(Ok(id)) => {
                        break Ok(UserEvent {
                            name: event_name_from_typ(&typ),
                            data: virtual_event_from_websys_event(event.clone(), target.clone()),
                            element: Some(ElementId(id)),
                            scope_id: None,
                            priority: dioxus_core::EventPriority::Medium,
                        });
                    }
                    Some(Err(e)) => {
                        break Err(e.into());
                    }
                    None => {
                        // walk the tree upwards until we actually find an event target
                        if let Some(parent) = target.parent_element() {
                            target = parent;
                        } else {
                            break Ok(UserEvent {
                                name: event_name_from_typ(&typ),
                                data: virtual_event_from_websys_event(
                                    event.clone(),
                                    target.clone(),
                                ),
                                element: None,
                                scope_id: None,
                                priority: dioxus_core::EventPriority::Low,
                            });
                        }
                    }
                }
            };

            if let Ok(synthetic_event) = decoded {
                // Try to prevent default if the attribute is set
                if let Some(node) = target.dyn_ref::<HtmlElement>() {
                    if let Some(name) = node.get_attribute("dioxus-prevent-default") {
                        if name == synthetic_event.name
                            || name.trim_start_matches("on") == synthetic_event.name
                        {
                            log::trace!("Preventing default");
                            event.prevent_default();
                        }
                    }
                }

                sender_callback.as_ref()(SchedulerMsg::Event(synthetic_event))
            }
        });

        let document = load_document();
        let root = match document.get_element_by_id(&cfg.rootname) {
            Some(root) => root,
            // a match here in order to avoid some error during runtime browser test
            None => {
                let body = document.create_element("body").ok().unwrap();
                body
            }
        };

        Self {
            interpreter: Interpreter::new(root.clone()),
            handler: Closure::wrap(callback),
            root,
        }
    }

    pub fn apply_edits(&mut self, mut edits: Vec<DomEdit>) {
        for edit in edits.drain(..) {
            match edit {
                DomEdit::PushRoot { root } => self.interpreter.PushRoot(root),
                DomEdit::AppendChildren { many } => self.interpreter.AppendChildren(many),
                DomEdit::ReplaceWith { root, m } => self.interpreter.ReplaceWith(root, m),
                DomEdit::InsertAfter { root, n } => self.interpreter.InsertAfter(root, n),
                DomEdit::InsertBefore { root, n } => self.interpreter.InsertBefore(root, n),
                DomEdit::Remove { root } => self.interpreter.Remove(root),

                DomEdit::CreateElement { tag, root } => self.interpreter.CreateElement(tag, root),
                DomEdit::CreateElementNs { tag, root, ns } => {
                    self.interpreter.CreateElementNs(tag, root, ns)
                }
                DomEdit::CreatePlaceholder { root } => self.interpreter.CreatePlaceholder(root),
                DomEdit::NewEventListener {
                    event_name, root, ..
                } => {
                    let handler: &Function = self.handler.as_ref().unchecked_ref();
                    self.interpreter.NewEventListener(event_name, root, handler);
                }

                DomEdit::RemoveEventListener { root, event } => {
                    self.interpreter.RemoveEventListener(root, event)
                }

                DomEdit::RemoveAttribute { root, name } => {
                    self.interpreter.RemoveAttribute(root, name)
                }

                DomEdit::CreateTextNode { text, root } => {
                    let text = serde_wasm_bindgen::to_value(text).unwrap();
                    self.interpreter.CreateTextNode(text, root)
                }
                DomEdit::SetText { root, text } => {
                    let text = serde_wasm_bindgen::to_value(text).unwrap();
                    self.interpreter.SetText(root, text)
                }
                DomEdit::SetAttribute {
                    root,
                    field,
                    value,
                    ns,
                } => {
                    let value = serde_wasm_bindgen::to_value(value).unwrap();
                    self.interpreter.SetAttribute(root, field, value, ns)
                }
            }
        }
    }
}

pub struct DioxusWebsysEvent(web_sys::Event);

// safety: currently the web is not multithreaded and our VirtualDom exists on the same thread
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for DioxusWebsysEvent {}
unsafe impl Sync for DioxusWebsysEvent {}

// todo: some of these events are being casted to the wrong event type.
// We need tests that simulate clicks/etc and make sure every event type works.
fn virtual_event_from_websys_event(
    event: web_sys::Event,
    target: Element,
) -> Arc<dyn Any + Send + Sync> {
    use dioxus_html::on::*;
    use dioxus_html::KeyCode;

    match event.type_().as_str() {
        "copy" | "cut" | "paste" => Arc::new(ClipboardData {}),
        "compositionend" | "compositionstart" | "compositionupdate" => {
            let evt: &web_sys::CompositionEvent = event.dyn_ref().unwrap();
            Arc::new(CompositionData {
                data: evt.data().unwrap_or_default(),
            })
        }
        "keydown" | "keypress" | "keyup" => {
            let evt: &web_sys::KeyboardEvent = event.dyn_ref().unwrap();
            Arc::new(KeyboardData {
                alt_key: evt.alt_key(),
                char_code: evt.char_code(),
                key: evt.key(),
                key_code: KeyCode::from_raw_code(evt.key_code() as u8),
                ctrl_key: evt.ctrl_key(),
                locale: "not implemented".to_string(),
                location: evt.location() as usize,
                meta_key: evt.meta_key(),
                repeat: evt.repeat(),
                shift_key: evt.shift_key(),
                which: evt.which() as usize,
            })
        }
        "focus" | "blur" => Arc::new(FocusData {}),

        // todo: these handlers might get really slow if the input box gets large and allocation pressure is heavy
        // don't have a good solution with the serialized event problem
        "change" | "input" | "invalid" | "reset" | "submit" => {
            let value: String = (&target)
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
                        let value: String = (&element)
                                .dyn_ref()
                                .map(|input: &web_sys::HtmlInputElement| {
                                    match input.type_().as_str() {
                                        "checkbox" => {
                                            match input.checked() {
                                                true => "true".to_string(),
                                                false => "false".to_string(),
                                            }
                                        },
                                        _ => input.value()
                                    }
                                })
                                .or_else(|| target.dyn_ref().map(|input: &web_sys::HtmlTextAreaElement| input.value()))
                                .or_else(|| target.dyn_ref().map(|input: &web_sys::HtmlSelectElement| input.value()))
                                .or_else(|| target.dyn_ref::<web_sys::HtmlElement>().unwrap().text_content())
                                .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener");

                        values.insert(name, value);
                    }
                }
            }

            Arc::new(FormData { value, values })
        }
        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            let evt: &web_sys::MouseEvent = event.dyn_ref().unwrap();
            Arc::new(MouseData {
                alt_key: evt.alt_key(),
                button: evt.button(),
                buttons: evt.buttons(),
                client_x: evt.client_x(),
                client_y: evt.client_y(),
                ctrl_key: evt.ctrl_key(),
                meta_key: evt.meta_key(),
                screen_x: evt.screen_x(),
                screen_y: evt.screen_y(),
                shift_key: evt.shift_key(),
                page_x: evt.page_x(),
                page_y: evt.page_y(),
            })
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            let evt: &web_sys::PointerEvent = event.dyn_ref().unwrap();
            Arc::new(PointerData {
                alt_key: evt.alt_key(),
                button: evt.button(),
                buttons: evt.buttons(),
                client_x: evt.client_x(),
                client_y: evt.client_y(),
                ctrl_key: evt.ctrl_key(),
                meta_key: evt.meta_key(),
                page_x: evt.page_x(),
                page_y: evt.page_y(),
                screen_x: evt.screen_x(),
                screen_y: evt.screen_y(),
                shift_key: evt.shift_key(),
                pointer_id: evt.pointer_id(),
                width: evt.width(),
                height: evt.height(),
                pressure: evt.pressure(),
                tangential_pressure: evt.tangential_pressure(),
                tilt_x: evt.tilt_x(),
                tilt_y: evt.tilt_y(),
                twist: evt.twist(),
                pointer_type: evt.pointer_type(),
                is_primary: evt.is_primary(),
                // get_modifier_state: evt.get_modifier_state(),
            })
        }
        "select" => Arc::new(SelectionData {}),
        "touchcancel" | "touchend" | "touchmove" | "touchstart" => {
            let evt: &web_sys::TouchEvent = event.dyn_ref().unwrap();
            Arc::new(TouchData {
                alt_key: evt.alt_key(),
                ctrl_key: evt.ctrl_key(),
                meta_key: evt.meta_key(),
                shift_key: evt.shift_key(),
            })
        }

        "scroll" => Arc::new(()),
        "wheel" => {
            let evt: &web_sys::WheelEvent = event.dyn_ref().unwrap();
            Arc::new(WheelData {
                delta_x: evt.delta_x(),
                delta_y: evt.delta_y(),
                delta_z: evt.delta_z(),
                delta_mode: evt.delta_mode(),
            })
        }
        "animationstart" | "animationend" | "animationiteration" => {
            let evt: &web_sys::AnimationEvent = event.dyn_ref().unwrap();
            Arc::new(AnimationData {
                elapsed_time: evt.elapsed_time(),
                animation_name: evt.animation_name(),
                pseudo_element: evt.pseudo_element(),
            })
        }
        "transitionend" => {
            let evt: &web_sys::TransitionEvent = event.dyn_ref().unwrap();
            Arc::new(TransitionData {
                elapsed_time: evt.elapsed_time(),
                property_name: evt.property_name(),
                pseudo_element: evt.pseudo_element(),
            })
        }
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => Arc::new(MediaData {}),
        "toggle" => Arc::new(ToggleData {}),

        _ => Arc::new(()),
    }
}

pub(crate) fn load_document() -> Document {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
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
