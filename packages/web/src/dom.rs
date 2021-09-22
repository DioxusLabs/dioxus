use std::{collections::HashMap, fmt::Debug, rc::Rc, sync::Arc};

use dioxus_core::{
    events::{on::GenericEventInner, SyntheticEvent, UserEvent},
    mutations::NodeRefMutation,
    scheduler::SchedulerMsg,
    DomEdit, ElementId, ScopeId,
};
use fxhash::FxHashMap;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{
    window, Attr, CssStyleDeclaration, Document, Element, Event, HtmlElement, HtmlInputElement,
    HtmlOptionElement, Node, NodeList, UiEvent,
};

use crate::{nodeslab::NodeSlab, WebConfig};

pub struct WebsysDom {
    stack: Stack,

    /// A map from ElementID (index) to Node
    nodes: NodeSlab,

    document: Document,

    root: Element,

    sender_callback: Rc<dyn Fn(SchedulerMsg)>,

    // map of listener types to number of those listeners
    // This is roughly a delegater
    // TODO: check how infero delegates its events - some are more performant
    listeners: FxHashMap<&'static str, (usize, Closure<dyn FnMut(&Event)>)>,

    // We need to make sure to add comments between text nodes
    // We ensure that the text siblings are patched by preventing the browser from merging
    // neighboring text nodes. Originally inspired by some of React's work from 2016.
    //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
    //  -> https://github.com/facebook/react/pull/5753
    last_node_was_text: bool,
}
impl WebsysDom {
    pub fn new(root: Element, cfg: WebConfig, sender_callback: Rc<dyn Fn(SchedulerMsg)>) -> Self {
        let document = load_document();

        let mut nodes = NodeSlab::new(2000);
        let mut listeners = FxHashMap::default();

        // re-hydrate the page - only supports one virtualdom per page
        if cfg.hydrate {
            // Load all the elements into the arena
            let node_list: NodeList = document.query_selector_all("dio_el").unwrap();
            let len = node_list.length() as usize;

            for x in 0..len {
                let node: Node = node_list.get(x as u32).unwrap();
                let el: &Element = node.dyn_ref::<Element>().unwrap();
                let id: String = el.get_attribute("dio_el").unwrap();
                let id = id.parse::<usize>().unwrap();
                nodes[id] = Some(node);
            }

            // Load all the event listeners into our listener register
            // TODO
        }

        let mut stack = Stack::with_capacity(10);
        let root_node = root.clone().dyn_into::<Node>().unwrap();
        stack.push(root_node);

        Self {
            stack,
            nodes,
            listeners,
            document,
            sender_callback,
            root,
            last_node_was_text: false,
        }
    }

    pub fn apply_refs(&mut self, refs: &[NodeRefMutation]) {
        for item in refs {
            if let Some(bla) = &item.element {
                let node = self.nodes[item.element_id.as_u64() as usize]
                    .as_ref()
                    .unwrap()
                    .clone();
                bla.set(Box::new(node)).unwrap();
            }
        }
    }

    pub fn process_edits(&mut self, edits: &mut Vec<DomEdit>) {
        for edit in edits.drain(..) {
            // log::info!("Handling edit: {:#?}", edit);
            match edit {
                DomEdit::PushRoot { id: root } => self.push(root),
                DomEdit::PopRoot => self.pop(),
                DomEdit::AppendChildren { many } => self.append_children(many),
                DomEdit::ReplaceWith { m, root } => self.replace_with(m, root),
                DomEdit::Remove { root } => self.remove(root),
                DomEdit::RemoveAllChildren => self.remove_all_children(),
                DomEdit::CreateTextNode { text, id } => self.create_text_node(text, id),
                DomEdit::CreateElement { tag, id } => self.create_element(tag, None, id),
                DomEdit::CreateElementNs { tag, id, ns } => self.create_element(tag, Some(ns), id),
                DomEdit::CreatePlaceholder { id } => self.create_placeholder(id),
                DomEdit::NewEventListener {
                    event_name,
                    scope,
                    mounted_node_id,
                } => self.new_event_listener(event_name, scope, mounted_node_id),

                DomEdit::RemoveEventListener { event } => todo!(),

                DomEdit::SetText { text } => self.set_text(text),
                DomEdit::SetAttribute { field, value, ns } => self.set_attribute(field, value, ns),
                DomEdit::RemoveAttribute { name } => self.remove_attribute(name),

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

    // drop the node off the stack
    fn pop(&mut self) {
        self.stack.pop();
    }

    fn append_children(&mut self, many: u32) {
        let root: Node = self
            .stack
            .list
            .get(self.stack.list.len() - (1 + many as usize))
            .unwrap()
            .clone();

        for child in self
            .stack
            .list
            .drain((self.stack.list.len() - many as usize)..)
        {
            if child.dyn_ref::<web_sys::Text>().is_some() {
                if self.last_node_was_text {
                    let comment_node = self
                        .document
                        .create_comment("dioxus")
                        .dyn_into::<Node>()
                        .unwrap();
                    root.append_child(&comment_node).unwrap();
                }
                self.last_node_was_text = true;
            } else {
                self.last_node_was_text = false;
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

    fn remove_all_children(&mut self) {
        todo!()
    }

    fn create_placeholder(&mut self, id: u64) {
        self.create_element("pre", None, id);
        self.set_attribute("hidden", "", None);
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

        self.stack.push(el.clone());
        self.nodes[(id as usize)] = Some(el);
    }

    fn new_event_listener(&mut self, event: &'static str, scope: ScopeId, real_id: u64) {
        let event = wasm_bindgen::intern(event);

        // attach the correct attributes to the element
        // these will be used by accessing the event's target
        // This ensures we only ever have one handler attached to the root, but decide
        // dynamically when we want to call a listener.

        let el = self.stack.top();

        let el = el
            .dyn_ref::<Element>()
            .expect(&format!("not an element: {:?}", el));

        // let scope_id = scope.data().as_ffi();
        let scope_id = scope.0 as u64;

        el.set_attribute(
            &format!("dioxus-event-{}", event),
            &format!("{}.{}", scope_id, real_id),
        )
        .unwrap();

        // el.set_attribute(&format!("dioxus-event"), &format!("{}", event))
        //     .unwrap();

        // Register the callback to decode

        if let Some(entry) = self.listeners.get_mut(event) {
            entry.0 += 1;
        } else {
            let trigger = self.sender_callback.clone();

            let handler = Closure::wrap(Box::new(move |event: &web_sys::Event| {
                // "Result" cannot be received from JS
                // Instead, we just build and immediately execute a closure that returns result
                match decode_trigger(event) {
                    Ok(synthetic_event) => trigger.as_ref()(SchedulerMsg::UiEvent(synthetic_event)),
                    Err(e) => log::error!("Error decoding Dioxus event attribute. {:#?}", e),
                };
            }) as Box<dyn FnMut(&Event)>);

            self.root
                .add_event_listener_with_callback(event, (&handler).as_ref().unchecked_ref())
                .unwrap();

            // Increment the listeners
            self.listeners.insert(event.into(), (1, handler));
        }
    }

    fn remove_event_listener(&mut self, event: &str) {
        todo!()
    }

    fn set_text(&mut self, text: &str) {
        self.stack.top().set_text_content(Some(text))
    }

    fn set_attribute(&mut self, name: &str, value: &str, ns: Option<&str>) {
        let node = self.stack.top();
        if let Some(el) = node.dyn_ref::<Element>() {
            match ns {
                // inline style support
                Some("style") => {
                    let el = el.dyn_ref::<HtmlElement>().unwrap();
                    let style_dc: CssStyleDeclaration = el.style();
                    style_dc.set_property(name, value).unwrap();
                }
                _ => el.set_attribute(name, value).unwrap(),
            }
        }

        if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
            if name == "value" {
                node.set_value(value);
            }
            if name == "checked" {
                node.set_checked(true);
            }
        }

        if let Some(node) = node.dyn_ref::<HtmlOptionElement>() {
            if name == "selected" {
                node.set_selected(true);
            }
        }
    }

    fn remove_attribute(&mut self, name: &str) {
        let node = self.stack.top();
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
        let after = self.nodes[root as usize].as_ref().unwrap();

        if n == 1 {
            let before = self.stack.pop();

            after
                .parent_node()
                .unwrap()
                .insert_before(&before, Some(&after))
                .unwrap();

            after.insert_before(&before, None).unwrap();
        } else {
            let arr: js_sys::Array = self
                .stack
                .list
                .drain((self.stack.list.len() - n as usize)..)
                .collect();

            if let Some(el) = after.dyn_ref::<Element>() {
                el.before_with_node(&arr).unwrap();
            } else if let Some(el) = after.dyn_ref::<web_sys::CharacterData>() {
                el.before_with_node(&arr).unwrap();
            } else if let Some(el) = after.dyn_ref::<web_sys::DocumentType>() {
                el.before_with_node(&arr).unwrap();
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    pub list: Vec<Node>,
}

impl Stack {
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Stack {
            list: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn push(&mut self, node: Node) {
        self.list.push(node);
    }

    #[inline]
    pub fn pop(&mut self) -> Node {
        self.list.pop().unwrap()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn top(&self) -> &Node {
        match self.list.last() {
            Some(a) => a,
            None => panic!("Called 'top' of an empty stack, make sure to push the root first"),
        }
    }
}

fn virtual_event_from_websys_event(event: web_sys::Event) -> SyntheticEvent {
    use crate::events::*;
    use dioxus_core::events::on::*;
    match event.type_().as_str() {
        "copy" | "cut" | "paste" => {
            SyntheticEvent::ClipboardEvent(ClipboardEvent(Rc::new(WebsysClipboardEvent(event))))
        }
        "compositionend" | "compositionstart" | "compositionupdate" => {
            let evt: web_sys::CompositionEvent = event.dyn_into().unwrap();
            SyntheticEvent::CompositionEvent(CompositionEvent(Rc::new(WebsysCompositionEvent(evt))))
        }
        "keydown" | "keypress" | "keyup" => {
            let evt: web_sys::KeyboardEvent = event.dyn_into().unwrap();
            SyntheticEvent::KeyboardEvent(KeyboardEvent(Rc::new(WebsysKeyboardEvent(evt))))
        }
        "focus" | "blur" => {
            let evt: web_sys::FocusEvent = event.dyn_into().unwrap();
            SyntheticEvent::FocusEvent(FocusEvent(Rc::new(WebsysFocusEvent(evt))))
        }
        "change" => {
            let evt = event.dyn_into().unwrap();
            SyntheticEvent::GenericEvent(GenericEvent(Rc::new(WebsysGenericUiEvent(evt))))
        }
        "input" | "invalid" | "reset" | "submit" => {
            let evt: web_sys::Event = event.dyn_into().unwrap();
            SyntheticEvent::FormEvent(FormEvent(Rc::new(WebsysFormEvent(evt))))
        }
        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            let evt: web_sys::MouseEvent = event.dyn_into().unwrap();
            SyntheticEvent::MouseEvent(MouseEvent(Rc::new(WebsysMouseEvent(evt))))
        }
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            let evt: web_sys::PointerEvent = event.dyn_into().unwrap();
            SyntheticEvent::PointerEvent(PointerEvent(Rc::new(WebsysPointerEvent(evt))))
        }
        "select" => {
            let evt: web_sys::UiEvent = event.dyn_into().unwrap();
            SyntheticEvent::SelectionEvent(SelectionEvent(Rc::new(WebsysGenericUiEvent(evt))))
        }
        "touchcancel" | "touchend" | "touchmove" | "touchstart" => {
            let evt: web_sys::TouchEvent = event.dyn_into().unwrap();
            SyntheticEvent::TouchEvent(TouchEvent(Rc::new(WebsysTouchEvent(evt))))
        }
        "scroll" => {
            let evt: web_sys::UiEvent = event.dyn_into().unwrap();
            SyntheticEvent::GenericEvent(GenericEvent(Rc::new(WebsysGenericUiEvent(evt))))
        }
        "wheel" => {
            let evt: web_sys::WheelEvent = event.dyn_into().unwrap();
            SyntheticEvent::WheelEvent(WheelEvent(Rc::new(WebsysWheelEvent(evt))))
        }
        "animationstart" | "animationend" | "animationiteration" => {
            let evt: web_sys::AnimationEvent = event.dyn_into().unwrap();
            SyntheticEvent::AnimationEvent(AnimationEvent(Rc::new(WebsysAnimationEvent(evt))))
        }
        "transitionend" => {
            let evt: web_sys::TransitionEvent = event.dyn_into().unwrap();
            SyntheticEvent::TransitionEvent(TransitionEvent(Rc::new(WebsysTransitionEvent(evt))))
        }
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => {
            let evt: web_sys::UiEvent = event.dyn_into().unwrap();
            SyntheticEvent::MediaEvent(MediaEvent(Rc::new(WebsysMediaEvent(evt))))
        }
        "toggle" => {
            let evt: web_sys::UiEvent = event.dyn_into().unwrap();
            SyntheticEvent::ToggleEvent(ToggleEvent(Rc::new(WebsysToggleEvent(evt))))
        }
        _ => {
            let evt: web_sys::UiEvent = event.dyn_into().unwrap();
            SyntheticEvent::GenericEvent(GenericEvent(Rc::new(WebsysGenericUiEvent(evt))))
        }
    }
}

/// This function decodes a websys event and produces an EventTrigger
/// With the websys implementation, we attach a unique key to the nodes
fn decode_trigger(event: &web_sys::Event) -> anyhow::Result<UserEvent> {
    let target = event
        .target()
        .expect("missing target")
        .dyn_into::<Element>()
        .expect("not a valid element");

    let typ = event.type_();

    log::debug!("Event type is {:?}", typ);

    let attrs = target.attributes();
    for x in 0..attrs.length() {
        let attr: Attr = attrs.item(x).unwrap();
        // log::debug!("attrs include: {:#?}, {:#?}", attr.name(), attr.value());
    }

    use anyhow::Context;

    // The error handling here is not very descriptive and needs to be replaced with a zero-cost error system
    let val: String = target
        .get_attribute(&format!("dioxus-event-{}", typ))
        .context(format!("wrong format - received {:#?}", typ))?;

    let mut fields = val.splitn(3, ".");

    let gi_id = fields
        .next()
        .and_then(|f| f.parse::<u64>().ok())
        .context("failed to parse gi id")?;

    let real_id = fields
        .next()
        .and_then(|raw_id| raw_id.parse::<u64>().ok())
        .context("failed to parse real id")?;

    // Call the trigger
    // log::debug!("decoded scope_id: {}, node_id: {:#?}", gi_id, real_id);

    let triggered_scope = gi_id;
    // let triggered_scope: ScopeId = KeyData::from_ffi(gi_id).into();
    // log::debug!("Triggered scope is {:#?}", triggered_scope);
    Ok(UserEvent {
        name: event_name_from_typ(&typ),
        event: virtual_event_from_websys_event(event.clone()),
        mounted_dom_id: Some(ElementId(real_id as usize)),
        scope: ScopeId(triggered_scope as usize),
    })
}

pub fn load_document() -> Document {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
}

pub fn event_name_from_typ(typ: &str) -> &'static str {
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
