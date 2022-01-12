//! Implementation of a renderer for Dioxus on the web.
//!
//! Oustanding todos:
//! - Removing event listeners (delegation)
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?>

use crate::bindings::Interpreter;
use dioxus_core::{DomEdit, ElementId, SchedulerMsg, ScopeId, UserEvent};
use fxhash::FxHashMap;
use std::{any::Any, fmt::Debug, rc::Rc, sync::Arc};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{
    CssStyleDeclaration, Document, Element, Event, HtmlElement, HtmlInputElement,
    HtmlOptionElement, HtmlTextAreaElement, Node,
};

use crate::{nodeslab::NodeSlab, WebConfig};

pub struct WebsysDom {
    document: Document,

    pub interpreter: Interpreter,

    pub(crate) root: Element,

    sender_callback: Rc<dyn Fn(SchedulerMsg)>,

    // map of listener types to number of those listeners
    // This is roughly a delegater
    // TODO: check how infero delegates its events - some are more performant
    listeners: FxHashMap<&'static str, ListenerEntry>,
}

type ListenerEntry = (usize, Closure<dyn FnMut(&Event)>);

impl WebsysDom {
    pub fn new(cfg: WebConfig, sender_callback: Rc<dyn Fn(SchedulerMsg)>) -> Self {
        let document = load_document();

        let listeners = FxHashMap::default();

        let mut stack = Stack::with_capacity(10);

        let root = load_document().get_element_by_id(&cfg.rootname).unwrap();
        let root_node = root.clone().dyn_into::<Node>().unwrap();
        stack.push(root_node);

        Self {
            interpreter: Interpreter::new(root.clone()),
            listeners,
            document,
            sender_callback,
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
                DomEdit::CreateTextNode { text, root } => {
                    self.interpreter.CreateTextNode(text, root)
                }
                DomEdit::CreateElement { tag, root } => self.interpreter.CreateElement(tag, root),
                DomEdit::CreateElementNs { tag, root, ns } => {
                    self.interpreter.CreateElementNs(tag, root, ns)
                }
                DomEdit::CreatePlaceholder { root } => self.interpreter.CreatePlaceholder(root),
                DomEdit::NewEventListener {
                    event_name,
                    scope,
                    root,
                } => self.interpreter.NewEventListener(event_name, scope.0, root),
                DomEdit::RemoveEventListener { root, event } => {
                    self.interpreter.RemoveEventListener(root, event)
                }
                DomEdit::SetText { root, text } => self.interpreter.SetText(root, text),
                DomEdit::SetAttribute {
                    root,
                    field,
                    value,
                    ns,
                } => self.interpreter.SetAttribute(root, field, value, ns),
                DomEdit::RemoveAttribute { root, name } => {
                    self.interpreter.RemoveAttribute(root, name)
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct Stack {
    list: Vec<Node>,
}

impl Stack {
    #[inline]
    fn with_capacity(cap: usize) -> Self {
        Stack {
            list: Vec::with_capacity(cap),
        }
    }

    #[inline]
    fn push(&mut self, node: Node) {
        self.list.push(node);
    }

    #[inline]
    fn pop(&mut self) -> Node {
        self.list.pop().unwrap()
    }

    fn top(&self) -> &Node {
        match self.list.last() {
            Some(a) => a,
            None => panic!("Called 'top' of an empty stack, make sure to push the root first"),
        }
    }
}

pub struct DioxusWebsysEvent(web_sys::Event);

// safety: currently the web is not multithreaded and our VirtualDom exists on the same thread
unsafe impl Send for DioxusWebsysEvent {}
unsafe impl Sync for DioxusWebsysEvent {}

// todo: some of these events are being casted to the wrong event type.
// We need tests that simulate clicks/etc and make sure every event type works.
fn virtual_event_from_websys_event(event: web_sys::Event) -> Arc<dyn Any + Send + Sync> {
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
            let evt: &web_sys::Event = event.dyn_ref().unwrap();

            let target: web_sys::EventTarget = evt.target().unwrap();
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

            Arc::new(FormData { value })
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

/// This function decodes a websys event and produces an EventTrigger
/// With the websys implementation, we attach a unique key to the nodes
fn decode_trigger(event: &web_sys::Event) -> anyhow::Result<UserEvent> {
    use anyhow::Context;

    let target = event
        .target()
        .expect("missing target")
        .dyn_into::<Element>()
        .expect("not a valid element");

    let typ = event.type_();

    let element_id = target
        .get_attribute("dioxus-id")
        .context("Could not find element id on event target")?
        .parse()?;

    Ok(UserEvent {
        name: event_name_from_typ(&typ),
        data: virtual_event_from_websys_event(event.clone()),
        element: Some(ElementId(element_id)),
        scope_id: None,
        priority: dioxus_core::EventPriority::Medium,
    })
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
