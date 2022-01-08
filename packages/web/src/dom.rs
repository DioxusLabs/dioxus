//! Implementation of a renderer for Dioxus on the web.
//!
//! Oustanding todos:
//! - Removing event listeners (delegation)
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?>

use dioxus_core::{DomEdit, ElementId, SchedulerMsg, ScopeId, UserEvent};
use fxhash::FxHashMap;
use std::{any::Any, fmt::Debug, rc::Rc, sync::Arc};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    CssStyleDeclaration, Document, Element, Event, HtmlElement, HtmlInputElement,
    HtmlOptionElement, HtmlTextAreaElement, Node,
};

use crate::{nodeslab::NodeSlab, WebConfig};

pub struct WebsysDom {
    stack: Stack,

    /// A map from ElementID (index) to Node
    pub(crate) nodes: NodeSlab,

    document: Document,

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

        let nodes = NodeSlab::new(2000);
        let listeners = FxHashMap::default();

        let mut stack = Stack::with_capacity(10);

        let root = load_document().get_element_by_id(&cfg.rootname).unwrap();
        let root_node = root.clone().dyn_into::<Node>().unwrap();
        stack.push(root_node);

        Self {
            stack,
            nodes,
            listeners,
            document,
            sender_callback,
            root,
        }
    }

    pub fn apply_edits(&mut self, mut edits: Vec<DomEdit>) {
        for edit in edits.drain(..) {
            match edit {
                DomEdit::PushRoot { root } => self.push(root),
                DomEdit::AppendChildren { many } => self.append_children(many),
                DomEdit::ReplaceWith { m, root } => self.replace_with(m, root),
                DomEdit::Remove { root } => self.remove(root),
                DomEdit::CreateTextNode { text, root: id } => self.create_text_node(text, id),
                DomEdit::CreateElement { tag, root: id } => self.create_element(tag, None, id),
                DomEdit::CreateElementNs { tag, root: id, ns } => {
                    self.create_element(tag, Some(ns), id)
                }
                DomEdit::CreatePlaceholder { root: id } => self.create_placeholder(id),
                DomEdit::NewEventListener {
                    event_name,
                    scope,
                    root: mounted_node_id,
                } => self.new_event_listener(event_name, scope, mounted_node_id),

                DomEdit::RemoveEventListener { event, root } => {
                    self.remove_event_listener(event, root)
                }

                DomEdit::SetText { text, root } => self.set_text(text, root),
                DomEdit::SetAttribute {
                    field,
                    value,
                    ns,
                    root,
                } => self.set_attribute(field, value, ns, root),
                DomEdit::RemoveAttribute { name, root } => self.remove_attribute(name, root),

                DomEdit::InsertAfter { n, root } => self.insert_after(n, root),
                DomEdit::InsertBefore { n, root } => self.insert_before(n, root),
            }
        }
    }
    fn push(&mut self, root: u64) {
        let key = root as usize;
        let domnode = &self.nodes[key];

        let real_node: Node = match domnode {
            Some(n) => n.clone(),
            None => todo!(),
        };

        self.stack.push(real_node);
    }

    fn append_children(&mut self, many: u32) {
        let root: Node = self
            .stack
            .list
            .get(self.stack.list.len() - (1 + many as usize))
            .unwrap()
            .clone();

        // We need to make sure to add comments between text nodes
        // We ensure that the text siblings are patched by preventing the browser from merging
        // neighboring text nodes. Originally inspired by some of React's work from 2016.
        //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
        //  -> https://github.com/facebook/react/pull/5753
        /*
        todo: we need to track this for replacing/insert after/etc
        */
        let mut last_node_was_text = false;

        for child in self
            .stack
            .list
            .drain((self.stack.list.len() - many as usize)..)
        {
            if child.dyn_ref::<web_sys::Text>().is_some() {
                if last_node_was_text {
                    let comment_node = self
                        .document
                        .create_comment("dioxus")
                        .dyn_into::<Node>()
                        .unwrap();
                    root.append_child(&comment_node).unwrap();
                }
                last_node_was_text = true;
            } else {
                last_node_was_text = false;
            }
            root.append_child(&child).unwrap();
        }
    }

    fn replace_with(&mut self, m: u32, root: u64) {
        let old = self.nodes[root as usize].as_ref().unwrap();

        let arr: js_sys::Array = self
            .stack
            .list
            .drain((self.stack.list.len() - m as usize)..)
            .collect();

        if let Some(el) = old.dyn_ref::<Element>() {
            el.replace_with_with_node(&arr).unwrap();
        } else if let Some(el) = old.dyn_ref::<web_sys::CharacterData>() {
            el.replace_with_with_node(&arr).unwrap();
        } else if let Some(el) = old.dyn_ref::<web_sys::DocumentType>() {
            el.replace_with_with_node(&arr).unwrap();
        }
    }

    fn remove(&mut self, root: u64) {
        let node = self.nodes[root as usize].as_ref().unwrap();
        if let Some(element) = node.dyn_ref::<Element>() {
            element.remove();
        } else {
            if let Some(parent) = node.parent_node() {
                parent.remove_child(&node).unwrap();
            }
        }
    }

    fn create_placeholder(&mut self, id: u64) {
        self.create_element("pre", None, id);
        self.set_attribute("hidden", "", None, id);
    }

    fn create_text_node(&mut self, text: &str, id: u64) {
        let textnode = self
            .document
            .create_text_node(text)
            .dyn_into::<Node>()
            .unwrap();

        self.stack.push(textnode.clone());

        self.nodes[(id as usize)] = Some(textnode);
    }

    fn create_element(&mut self, tag: &str, ns: Option<&'static str>, id: u64) {
        let tag = wasm_bindgen::intern(tag);

        let el = match ns {
            Some(ns) => self
                .document
                .create_element_ns(Some(ns), tag)
                .unwrap()
                .dyn_into::<Node>()
                .unwrap(),
            None => self
                .document
                .create_element(tag)
                .unwrap()
                .dyn_into::<Node>()
                .unwrap(),
        };

        use smallstr::SmallString;
        use std::fmt::Write;

        let mut s: SmallString<[u8; 8]> = smallstr::SmallString::new();
        write!(s, "{}", id).unwrap();

        let el2 = el.dyn_ref::<Element>().unwrap();
        el2.set_attribute("dioxus-id", s.as_str()).unwrap();

        self.stack.push(el.clone());
        self.nodes[(id as usize)] = Some(el);
    }

    fn new_event_listener(&mut self, event: &'static str, _scope: ScopeId, _real_id: u64) {
        let event = wasm_bindgen::intern(event);

        // attach the correct attributes to the element
        // these will be used by accessing the event's target
        // This ensures we only ever have one handler attached to the root, but decide
        // dynamically when we want to call a listener.

        let el = self.stack.top();

        let el = el.dyn_ref::<Element>().unwrap();

        el.set_attribute("dioxus-event", event).unwrap();

        // Register the callback to decode

        if let Some(entry) = self.listeners.get_mut(event) {
            entry.0 += 1;
        } else {
            let trigger = self.sender_callback.clone();

            let c: Box<dyn FnMut(&Event)> = Box::new(move |event: &web_sys::Event| {
                // "Result" cannot be received from JS
                // Instead, we just build and immediately execute a closure that returns result
                match decode_trigger(event) {
                    Ok(synthetic_event) => {
                        let target = event.target().unwrap();
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

                        trigger.as_ref()(SchedulerMsg::Event(synthetic_event))
                    }
                    Err(e) => log::error!("Error decoding Dioxus event attribute. {:#?}", e),
                };
            });

            let handler = Closure::wrap(c);

            self.root
                .add_event_listener_with_callback(event, (&handler).as_ref().unchecked_ref())
                .unwrap();

            // Increment the listeners
            self.listeners.insert(event.into(), (1, handler));
        }
    }

    fn remove_event_listener(&mut self, _event: &str, _root: u64) {
        todo!()
    }

    fn set_text(&mut self, text: &str, root: u64) {
        let el = self.nodes[root as usize].as_ref().unwrap();
        el.set_text_content(Some(text))
    }

    fn set_attribute(&mut self, name: &str, value: &str, ns: Option<&str>, root: u64) {
        let node = self.nodes[root as usize].as_ref().unwrap();
        if ns == Some("style") {
            if let Some(el) = node.dyn_ref::<Element>() {
                let el = el.dyn_ref::<HtmlElement>().unwrap();
                let style_dc: CssStyleDeclaration = el.style();
                style_dc.set_property(name, value).unwrap();
            }
        } else {
            let fallback = || {
                let el = node.dyn_ref::<Element>().unwrap();
                el.set_attribute(name, value).unwrap()
            };
            match name {
                "dangerous_inner_html" => {
                    if let Some(el) = node.dyn_ref::<Element>() {
                        el.set_inner_html(value);
                    }
                }
                "value" => {
                    if let Some(input) = node.dyn_ref::<HtmlInputElement>() {
                        /*
                        if the attribute being set is the same as the value of the input, then don't bother setting it.
                        This is used in controlled components to keep the cursor in the right spot.

                        this logic should be moved into the virtualdom since we have the notion of "volatile"
                        */
                        if input.value() != value {
                            input.set_value(value);
                        }
                    } else if let Some(node) = node.dyn_ref::<HtmlTextAreaElement>() {
                        if name == "value" {
                            node.set_value(value);
                        }
                    } else {
                        fallback();
                    }
                }
                "checked" => {
                    if let Some(input) = node.dyn_ref::<HtmlInputElement>() {
                        match value {
                            "true" => input.set_checked(true),
                            "false" => input.set_checked(false),
                            _ => fallback(),
                        }
                    } else {
                        fallback();
                    }
                }
                "selected" => {
                    if let Some(node) = node.dyn_ref::<HtmlOptionElement>() {
                        node.set_selected(true);
                    } else {
                        fallback();
                    }
                }
                _ => {
                    // https://github.com/facebook/react/blob/8b88ac2592c5f555f315f9440cbb665dd1e7457a/packages/react-dom/src/shared/DOMProperty.js#L352-L364
                    if value == "false" {
                        if let Some(el) = node.dyn_ref::<Element>() {
                            match name {
                                "allowfullscreen"
                                | "allowpaymentrequest"
                                | "async"
                                | "autofocus"
                                | "autoplay"
                                | "checked"
                                | "controls"
                                | "default"
                                | "defer"
                                | "disabled"
                                | "formnovalidate"
                                | "hidden"
                                | "ismap"
                                | "itemscope"
                                | "loop"
                                | "multiple"
                                | "muted"
                                | "nomodule"
                                | "novalidate"
                                | "open"
                                | "playsinline"
                                | "readonly"
                                | "required"
                                | "reversed"
                                | "selected"
                                | "truespeed" => {
                                    let _ = el.remove_attribute(name);
                                }
                                _ => {
                                    let _ = el.set_attribute(name, value);
                                }
                            };
                        }
                    } else {
                        fallback();
                    }
                }
            }
        }
    }

    fn remove_attribute(&mut self, name: &str, root: u64) {
        let node = self.nodes[root as usize].as_ref().unwrap();
        if let Some(node) = node.dyn_ref::<web_sys::Element>() {
            node.remove_attribute(name).unwrap();
        }
        if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
            // Some attributes are "volatile" and don't work through `removeAttribute`.
            if name == "value" {
                node.set_value("");
            }
            if name == "checked" {
                node.set_checked(false);
            }
        }

        if let Some(node) = node.dyn_ref::<HtmlOptionElement>() {
            if name == "selected" {
                node.set_selected(true);
            }
        }
    }

    fn insert_after(&mut self, n: u32, root: u64) {
        let old = self.nodes[root as usize].as_ref().unwrap();

        let arr: js_sys::Array = self
            .stack
            .list
            .drain((self.stack.list.len() - n as usize)..)
            .collect();

        if let Some(el) = old.dyn_ref::<Element>() {
            el.after_with_node(&arr).unwrap();
        } else if let Some(el) = old.dyn_ref::<web_sys::CharacterData>() {
            el.after_with_node(&arr).unwrap();
        } else if let Some(el) = old.dyn_ref::<web_sys::DocumentType>() {
            el.after_with_node(&arr).unwrap();
        }
    }

    fn insert_before(&mut self, n: u32, root: u64) {
        let anchor = self.nodes[root as usize].as_ref().unwrap();

        if n == 1 {
            let before = self.stack.pop();

            anchor
                .parent_node()
                .unwrap()
                .insert_before(&before, Some(&anchor))
                .unwrap();
        } else {
            let arr: js_sys::Array = self
                .stack
                .list
                .drain((self.stack.list.len() - n as usize)..)
                .collect();

            if let Some(el) = anchor.dyn_ref::<Element>() {
                el.before_with_node(&arr).unwrap();
            } else if let Some(el) = anchor.dyn_ref::<web_sys::CharacterData>() {
                el.before_with_node(&arr).unwrap();
            } else if let Some(el) = anchor.dyn_ref::<web_sys::DocumentType>() {
                el.before_with_node(&arr).unwrap();
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
