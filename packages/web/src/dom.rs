use std::{collections::HashMap, rc::Rc, sync::Arc};

use dioxus_core::{
    events::{EventTrigger, VirtualEvent},
    DomEdit, ElementId, ScopeId,
};
use fxhash::FxHashMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    window, Document, Element, Event, HtmlElement, HtmlInputElement, HtmlOptionElement, Node,
    NodeList,
};

use crate::{nodeslab::NodeSlab, WebConfig};

pub struct WebsysDom {
    stack: Stack,

    /// A map from ElementID (index) to Node
    nodes: NodeSlab,

    document: Document,

    root: Element,

    event_receiver: async_channel::Receiver<EventTrigger>,

    trigger: Arc<dyn Fn(EventTrigger)>,

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
    pub fn new(root: Element, cfg: WebConfig) -> Self {
        let document = load_document();

        let (sender, receiver) = async_channel::unbounded::<EventTrigger>();

        let sender_callback = Arc::new(move |ev| {
            let c = sender.clone();
            wasm_bindgen_futures::spawn_local(async move {
                log::debug!("sending event through channel");
                c.send(ev).await.unwrap();
            });
        });

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

                // this autoresizes the vector if needed
                nodes[id] = Some(node);
            }

            // Load all the event listeners into our listener register
        }

        let mut stack = Stack::with_capacity(10);
        let root_node = root.clone().dyn_into::<Node>().unwrap();
        stack.push(root_node);

        Self {
            stack,
            nodes,
            listeners,
            document,
            event_receiver: receiver,
            trigger: sender_callback,
            root,
            last_node_was_text: false,
        }
    }

    pub async fn wait_for_event(&mut self) -> Option<EventTrigger> {
        let v = self.event_receiver.recv().await.unwrap();
        Some(v)
    }

    pub fn process_edits(&mut self, edits: &mut Vec<DomEdit>) {
        for edit in edits.drain(..) {
            log::info!("Handling edit: {:#?}", edit);
            match edit {
                DomEdit::PushRoot { id: root } => self.push(root),
                DomEdit::PopRoot => self.pop(),
                DomEdit::AppendChildren { many } => self.append_children(many),
                DomEdit::ReplaceWith { n, m } => self.replace_with(n, m),
                DomEdit::Remove => self.remove(),
                DomEdit::RemoveAllChildren => self.remove_all_children(),
                DomEdit::CreateTextNode { text, id } => self.create_text_node(text, id),
                DomEdit::CreateElement { tag, id } => self.create_element(tag, None, id),
                DomEdit::CreateElementNs { tag, id, ns } => self.create_element(tag, Some(ns), id),
                DomEdit::CreatePlaceholder { id } => self.create_placeholder(id),
                DomEdit::NewEventListener {
                    event_name: event,
                    scope,
                    mounted_node_id: node,
                } => self.new_event_listener(event, scope, node),

                DomEdit::RemoveEventListener { event } => todo!(),
                DomEdit::SetText { text } => self.set_text(text),
                DomEdit::SetAttribute { field, value, ns } => self.set_attribute(field, value, ns),
                DomEdit::RemoveAttribute { name } => self.remove_attribute(name),

                DomEdit::InsertAfter { n } => self.insert_after(n),
                DomEdit::InsertBefore { n } => self.insert_before(n),
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
        log::debug!("Called [`append_child`]");

        let root: Node = self
            .stack
            .list
            .get(self.stack.list.len() - (1 + many as usize))
            .unwrap()
            .clone();

        for _ in 0..many {
            let child = self.stack.pop();

            if child.dyn_ref::<web_sys::Text>().is_some() {
                if self.last_node_was_text {
                    let comment_node = self
                        .document
                        .create_comment("dioxus")
                        .dyn_into::<Node>()
                        .unwrap();
                    self.stack.top().append_child(&comment_node).unwrap();
                }
                self.last_node_was_text = true;
            } else {
                self.last_node_was_text = false;
            }

            root.append_child(&child).unwrap();
        }
    }

    fn replace_with(&mut self, n: u32, m: u32) {
        log::debug!("Called [`replace_with`]");

        let mut new_nodes = vec![];
        for _ in 0..m {
            new_nodes.push(self.stack.pop());
        }

        let mut old_nodes = vec![];
        for _ in 0..n {
            old_nodes.push(self.stack.pop());
        }

        let old = old_nodes[0].clone();
        let arr: js_sys::Array = new_nodes.iter().collect();
        let el = old.dyn_into::<Element>().unwrap();
        el.replace_with_with_node(&arr).unwrap();
        // let arr = js_sys::Array::from();

        // TODO: use different-sized replace withs
        // if m == 1 {
        //     if old_node.has_type::<Element>() {
        //         old_node
        //             .dyn_ref::<Element>()
        //             .unwrap()
        //             .replace_with_with_node_1(&new_node)
        //             .unwrap();
        //     } else if old_node.has_type::<web_sys::CharacterData>() {
        //         old_node
        //             .dyn_ref::<web_sys::CharacterData>()
        //             .unwrap()
        //             .replace_with_with_node_1(&new_node)
        //             .unwrap();
        //     } else if old_node.has_type::<web_sys::DocumentType>() {
        //         old_node
        //             .dyn_ref::<web_sys::DocumentType>()
        //             .unwrap()
        //             .replace_with_with_node_1(&new_node)
        //             .unwrap();
        //     } else {
        //         panic!("Cannot replace node: {:?}", old_node);
        //     }
        // }

        // self.stack.push(new_node);
    }

    fn remove(&mut self) {
        log::debug!("Called [`remove`]");
        todo!()
    }

    fn remove_all_children(&mut self) {
        log::debug!("Called [`remove_all_children`]");
        todo!()
    }

    fn create_placeholder(&mut self, id: u64) {
        self.create_element("pre", None, id);
        self.set_attribute("hidden", "", None);
    }
    fn create_text_node(&mut self, text: &str, id: u64) {
        // let nid = self.node_counter.next();

        let textnode = self
            .document
            .create_text_node(text)
            .dyn_into::<Node>()
            .unwrap();

        let id = id as usize;
        self.stack.push(textnode.clone());
        self.nodes[id] = Some(textnode);
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
        let id = id as usize;

        self.stack.push(el.clone());
        self.nodes[id] = Some(el);
        // let nid = self.node_counter.?next();
        // let nid = self.nodes.insert(el).data().as_ffi();
        // log::debug!("Called [`create_element`]: {}, {:?}", tag, nid);
        // ElementId::new(nid)
    }

    fn new_event_listener(&mut self, event: &'static str, scope: ScopeId, real_id: u64) {
        // let (_on, event) = event.split_at(2);
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

        el.set_attribute(&format!("dioxus-event"), &format!("{}", event))
            .unwrap();

        // Register the callback to decode

        if let Some(entry) = self.listeners.get_mut(event) {
            entry.0 += 1;
        } else {
            let trigger = self.trigger.clone();

            let handler = Closure::wrap(Box::new(move |event: &web_sys::Event| {
                // "Result" cannot be received from JS
                // Instead, we just build and immediately execute a closure that returns result
                match decode_trigger(event) {
                    Ok(synthetic_event) => trigger.as_ref()(synthetic_event),
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
        if name == "class" {
            match ns {
                Some("http://www.w3.org/2000/svg") => {
                    //
                    if let Some(el) = self.stack.top().dyn_ref::<web_sys::SvgElement>() {
                        let r: web_sys::SvgAnimatedString = el.class_name();
                        r.set_base_val(value);
                        // el.set_class_name(value);
                    }
                }
                _ => {
                    if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                        el.set_class_name(value);
                    }
                }
            }
        } else {
            if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                el.set_attribute(name, value).unwrap();
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

    fn insert_after(&mut self, n: u32) {
        let mut new_nodes = vec![];
        for _ in 0..n {
            new_nodes.push(self.stack.pop());
        }

        let after = self.stack.top().clone();
        let arr: js_sys::Array = new_nodes.iter().collect();

        let el = after.dyn_into::<Element>().unwrap();
        el.after_with_node(&arr).unwrap();
        // let mut old_nodes = vec![];
        // for _ in 0..n {
        //     old_nodes.push(self.stack.pop());
        // }

        // let el = self.stack.top();
    }

    fn insert_before(&mut self, n: u32) {
        let n = n as usize;
        let root = self
            .stack
            .list
            .get(self.stack.list.len() - n)
            .unwrap()
            .clone();
        for _ in 0..n {
            let el = self.stack.pop();
            root.insert_before(&el, None).unwrap();
        }
    }
}

impl<'a> dioxus_core::diff::RealDom<'a> for WebsysDom {
    // fn request_available_node(&mut self) -> ElementId {
    //     let key = self.nodes.insert(None);
    //     log::debug!("making new key: {:#?}", key);
    //     ElementId(key.data().as_ffi())
    // }

    fn raw_node_as_any(&self) -> &mut dyn std::any::Any {
        todo!()
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

fn virtual_event_from_websys_event(event: &web_sys::Event) -> VirtualEvent {
    use dioxus_core::events::on::*;
    match event.type_().as_str() {
        "copy" | "cut" | "paste" => {
            // let evt: web_sys::ClipboardEvent = event.clone().dyn_into().unwrap();

            todo!()
        }

        "compositionend" | "compositionstart" | "compositionupdate" => {
            let evt: web_sys::CompositionEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "keydown" | "keypress" | "keyup" => {
            let evt: web_sys::KeyboardEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "focus" | "blur" => {
            let evt: web_sys::FocusEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "change" => {
            let evt: web_sys::Event = event.clone().dyn_into().expect("wrong error typ");
            todo!()
            // VirtualEvent::FormEvent(FormEvent {value:})
        }

        "input" | "invalid" | "reset" | "submit" => {
            // is a special react events
            let evt: web_sys::InputEvent = event.clone().dyn_into().expect("wrong event type");
            let this: web_sys::EventTarget = evt.target().unwrap();

            let value = (&this)
                .dyn_ref()
                .map(|input: &web_sys::HtmlInputElement| input.value())
                .or_else(|| {
                    (&this)
                        .dyn_ref()
                        .map(|input: &web_sys::HtmlTextAreaElement| input.value())
                })
                .or_else(|| {
                    (&this)
                        .dyn_ref::<web_sys::HtmlElement>()
                        .unwrap()
                        .text_content()
                })
                .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener");

            // let p2 = evt.data_transfer();

            // let value: Option<String> = (&evt).data();
            // let value = val;
            // let value = value.unwrap_or_default();
            // let value = (&evt).data().expect("No data to unwrap");

            // todo - this needs to be a "controlled" event
            // these events won't carry the right data with them
            todo!()
            // VirtualEvent::FormEvent(FormEvent { value })
        }

        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            let evt: web_sys::MouseEvent = event.clone().dyn_into().unwrap();

            #[derive(Debug)]
            pub struct CustomMouseEvent(web_sys::MouseEvent);
            impl dioxus_core::events::on::MouseEventInner for CustomMouseEvent {
                fn alt_key(&self) -> bool {
                    self.0.alt_key()
                }
                fn button(&self) -> i16 {
                    self.0.button()
                }
                fn buttons(&self) -> u16 {
                    self.0.buttons()
                }
                fn client_x(&self) -> i32 {
                    self.0.client_x()
                }
                fn client_y(&self) -> i32 {
                    self.0.client_y()
                }
                fn ctrl_key(&self) -> bool {
                    self.0.ctrl_key()
                }
                fn meta_key(&self) -> bool {
                    self.0.meta_key()
                }
                fn page_x(&self) -> i32 {
                    self.0.page_x()
                }
                fn page_y(&self) -> i32 {
                    self.0.page_y()
                }
                fn screen_x(&self) -> i32 {
                    self.0.screen_x()
                }
                fn screen_y(&self) -> i32 {
                    self.0.screen_y()
                }
                fn shift_key(&self) -> bool {
                    self.0.shift_key()
                }

                // yikes
                // https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
                fn get_modifier_state(&self, key_code: &str) -> bool {
                    self.0.get_modifier_state(key_code)
                }
            }
            VirtualEvent::MouseEvent(MouseEvent(Rc::new(CustomMouseEvent(evt))))
        }

        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            let evt: web_sys::PointerEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "select" => {
            // let evt: web_sys::SelectionEvent = event.clone().dyn_into().unwrap();
            // not required to construct anything special beyond standard event stuff
            todo!()
        }

        "touchcancel" | "touchend" | "touchmove" | "touchstart" => {
            let evt: web_sys::TouchEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "scroll" => {
            // let evt: web_sys::UIEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "wheel" => {
            let evt: web_sys::WheelEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => {
            // not required to construct anything special beyond standard event stuff

            // let evt: web_sys::MediaEvent = event.clone().dyn_into().unwrap();
            // let evt: web_sys::MediaEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "animationstart" | "animationend" | "animationiteration" => {
            let evt: web_sys::AnimationEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "transitionend" => {
            let evt: web_sys::TransitionEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "toggle" => {
            // not required to construct anything special beyond standard event stuff (target)

            // let evt: web_sys::ToggleEvent = event.clone().dyn_into().unwrap();
            todo!()
        }
        _ => {
            todo!()
        }
    }
}

/// This function decodes a websys event and produces an EventTrigger
/// With the websys implementation, we attach a unique key to the nodes
fn decode_trigger(event: &web_sys::Event) -> anyhow::Result<EventTrigger> {
    log::debug!("Handling event!");

    let target = event
        .target()
        .expect("missing target")
        .dyn_into::<Element>()
        .expect("not a valid element");

    let typ = event.type_();

    use anyhow::Context;

    let attrs = target.attributes();
    for x in 0..attrs.length() {
        let attr = attrs.item(x).unwrap();
        log::debug!("attrs include: {:#?}", attr);
    }

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
    log::debug!("decoded scope_id: {}, node_id: {:#?}", gi_id, real_id);

    let triggered_scope = gi_id;
    // let triggered_scope: ScopeId = KeyData::from_ffi(gi_id).into();
    log::debug!("Triggered scope is {:#?}", triggered_scope);
    Ok(EventTrigger::new(
        virtual_event_from_websys_event(event),
        ScopeId(triggered_scope as usize),
        Some(ElementId(real_id as usize)),
        dioxus_core::events::EventPriority::High,
    ))
}

pub fn prepare_websys_dom() -> Element {
    load_document().get_element_by_id("dioxusroot").unwrap()
}

pub fn load_document() -> Document {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
}
