//! Implementation of a renderer for Dioxus on the web.
//!
//! Oustanding todos:
//! - Removing event listeners (delegation)
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?>

use dioxus_core::{
    BorrowedAttributeValue, ElementId, Mutation, Template, TemplateAttribute, TemplateNode,
};
use dioxus_html::{event_bubbles, CompositionData, FormData};
use dioxus_interpreter_js::{save_template, Channel};
use futures_channel::mpsc;
use rustc_hash::FxHashMap;
use std::{any::Any, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

use crate::Config;

pub struct WebsysDom {
    document: Document,
    templates: FxHashMap<String, u32>,
    max_template_id: u32,
    interpreter: Channel,
}

pub struct UiEvent {
    pub name: String,
    pub bubbles: bool,
    pub element: ElementId,
    pub data: Rc<dyn Any>,
    pub event: Event,
}

impl WebsysDom {
    pub fn new(cfg: Config, event_channel: mpsc::UnboundedSender<UiEvent>) -> Self {
        // eventually, we just want to let the interpreter do all the work of decoding events into our event type
        // a match here in order to avoid some error during runtime browser test
        let document = load_document();
        let root = match document.get_element_by_id(&cfg.rootname) {
            Some(root) => root,
            None => document.create_element("body").ok().unwrap(),
        };
        let interpreter = Channel::default();

        let handler: Closure<dyn FnMut(&Event)> =
            Closure::wrap(Box::new(move |event: &web_sys::Event| {
                let name = event.type_();
                let element = walk_event_for_id(event);
                let bubbles = dioxus_html::event_bubbles(name.as_str());
                if let Some((element, target)) = element {
                    if target
                        .get_attribute("dioxus-prevent-default")
                        .as_deref()
                        .map(|f| f.trim_start_matches("on"))
                        == Some(&name)
                    {
                        event.prevent_default();
                    }

                    let data = virtual_event_from_websys_event(event.clone(), target);
                    let _ = event_channel.unbounded_send(UiEvent {
                        name,
                        bubbles,
                        element,
                        data,
                        event: event.clone(),
                    });
                }
            }));

        dioxus_interpreter_js::initilize(root.unchecked_into(), handler.as_ref().unchecked_ref());
        handler.forget();
        Self {
            document,
            interpreter,
            templates: FxHashMap::default(),
            max_template_id: 0,
        }
    }

    pub fn mount(&mut self) {
        self.interpreter.mount_to_root();
    }

    pub fn load_templates(&mut self, templates: &[Template]) {
        for template in templates {
            let mut roots = vec![];

            for root in template.roots {
                roots.push(self.create_template_node(root))
            }

            self.templates
                .insert(template.name.to_owned(), self.max_template_id);
            save_template(roots, self.max_template_id);
            self.max_template_id += 1
        }
    }

    fn create_template_node(&self, v: &TemplateNode) -> web_sys::Node {
        use TemplateNode::*;
        match v {
            Element {
                tag,
                namespace,
                attrs,
                children,
                ..
            } => {
                let el = match namespace {
                    Some(ns) => self.document.create_element_ns(Some(ns), tag).unwrap(),
                    None => self.document.create_element(tag).unwrap(),
                };
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } = attr
                    {
                        match namespace {
                            Some(ns) if *ns == "style" => {
                                el.dyn_ref::<HtmlElement>()
                                    .map(|f| f.style().set_property(name, value));
                            }
                            Some(ns) => el.set_attribute_ns(Some(ns), name, value).unwrap(),
                            None => el.set_attribute(name, value).unwrap(),
                        }
                    }
                }
                for child in *children {
                    let _ = el.append_child(&self.create_template_node(child));
                }
                el.dyn_into().unwrap()
            }
            Text { text } => self.document.create_text_node(text).dyn_into().unwrap(),
            DynamicText { .. } => self.document.create_text_node("p").dyn_into().unwrap(),
            Dynamic { .. } => {
                let el = self.document.create_element("pre").unwrap();
                let _ = el.toggle_attribute("hidden");
                el.dyn_into().unwrap()
            }
        }
    }

    pub fn apply_edits(&mut self, mut edits: Vec<Mutation>) {
        use Mutation::*;
        let i = &mut self.interpreter;
        for edit in &edits {
            match edit {
                AppendChildren { id, m } => i.append_children(id.0 as u32, *m as u32),
                AssignId { path, id } => {
                    i.assign_id(path.as_ptr() as u32, path.len() as u8, id.0 as u32)
                }
                CreatePlaceholder { id } => i.create_placeholder(id.0 as u32),
                CreateTextNode { value, id } => i.create_text_node(value, id.0 as u32),
                HydrateText { path, value, id } => {
                    i.hydrate_text(path.as_ptr() as u32, path.len() as u8, value, id.0 as u32)
                }
                LoadTemplate { name, index, id } => {
                    if let Some(tmpl_id) = self.templates.get(*name) {
                        i.load_template(*tmpl_id, *index as u32, id.0 as u32)
                    }
                }
                ReplaceWith { id, m } => i.replace_with(id.0 as u32, *m as u32),
                ReplacePlaceholder { path, m } => {
                    i.replace_placeholder(path.as_ptr() as u32, path.len() as u8, *m as u32)
                }
                InsertAfter { id, m } => i.insert_after(id.0 as u32, *m as u32),
                InsertBefore { id, m } => i.insert_before(id.0 as u32, *m as u32),
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => match value {
                    BorrowedAttributeValue::Text(txt) => {
                        i.set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
                    }
                    BorrowedAttributeValue::Float(f) => {
                        i.set_attribute(id.0 as u32, name, &f.to_string(), ns.unwrap_or_default())
                    }
                    BorrowedAttributeValue::Int(n) => {
                        i.set_attribute(id.0 as u32, name, &n.to_string(), ns.unwrap_or_default())
                    }
                    BorrowedAttributeValue::Bool(b) => i.set_attribute(
                        id.0 as u32,
                        name,
                        if *b { "true" } else { "false" },
                        ns.unwrap_or_default(),
                    ),
                    BorrowedAttributeValue::None => {
                        i.remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
                    }
                    _ => unreachable!(),
                },
                SetText { value, id } => i.set_text(id.0 as u32, value),
                NewEventListener { name, id, .. } => {
                    i.new_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
                }
                RemoveEventListener { name, id } => {
                    i.remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
                }
                Remove { id } => i.remove(id.0 as u32),
                PushRoot { id } => i.push_root(id.0 as u32),
            }
        }
        edits.clear();
        i.flush();
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

        "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            Rc::new(MouseData::from(event))
        }
        "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart"
        | "drop" => {
            let mouse = MouseData::from(event);
            Rc::new(DragData { mouse })
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

fn walk_event_for_id(event: &web_sys::Event) -> Option<(ElementId, web_sys::Element)> {
    let mut target = event
        .target()
        .expect("missing target")
        .dyn_into::<web_sys::Element>()
        .expect("not a valid element");

    loop {
        match target.get_attribute("data-dioxus-id").map(|f| f.parse()) {
            Some(Ok(id)) => return Some((ElementId(id), target)),
            Some(Err(_)) => return None,

            // walk the tree upwards until we actually find an event target
            None => match target.parent_element() {
                Some(parent) => target = parent,
                None => return None,
            },
        }
    }
}
