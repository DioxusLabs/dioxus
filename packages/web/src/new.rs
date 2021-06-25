use std::{collections::HashMap, rc::Rc, sync::Arc};

use dioxus_core::{
    events::{EventTrigger, VirtualEvent},
    prelude::ScopeIdx,
    virtual_dom::RealDomNode,
};
use fxhash::FxHashMap;
use nohash_hasher::IntMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    window, Document, Element, Event, HtmlElement, HtmlInputElement, HtmlOptionElement, Node,
};

#[derive(Debug, Clone)]
pub struct DomNode {
    node: Node,
    meta: NodeMetadata,
}
impl std::ops::Deref for DomNode {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DomNode {
    pub fn new_root(node: Node) -> Self {
        Self {
            node,
            meta: NodeMetadata::IsRoot,
        }
    }
    pub fn new_style(node: Node) -> Self {
        Self {
            node,
            meta: NodeMetadata::IsStyle,
        }
    }
    pub fn new_text(node: Node) -> Self {
        Self {
            node,
            meta: NodeMetadata::IsText,
        }
    }
    pub fn new_nothing(node: Node) -> Self {
        Self {
            node,
            meta: NodeMetadata::Nothing,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NodeMetadata {
    IsStyle,
    IsText,
    IsRoot,
    Nothing,
}

pub struct WebsysDom {
    pub stack: Stack,
    nodes: IntMap<u32, DomNode>,
    document: Document,
    root: Element,

    event_receiver: async_channel::Receiver<EventTrigger>,
    trigger: Arc<dyn Fn(EventTrigger)>,

    // every callback gets a monotomically increasing callback ID
    callback_id: usize,

    // map of listener types to number of those listeners
    listeners: FxHashMap<String, (usize, Closure<dyn FnMut(&Event)>)>,

    // Map of callback_id to component index and listener id
    callback_map: FxHashMap<usize, (usize, usize)>,

    // We need to make sure to add comments between text nodes
    // We ensure that the text siblings are patched by preventing the browser from merging
    // neighboring text nodes. Originally inspired by some of React's work from 2016.
    //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
    //  -> https://github.com/facebook/react/pull/5753
    //
    // `ptns` = Percy text node separator
    // TODO
    last_node_was_text: bool,

    // used to support inline styles
    building_style: bool,

    node_counter: Counter,
}
impl WebsysDom {
    pub fn new(root: Element) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        let (sender, mut receiver) = async_channel::unbounded::<EventTrigger>();

        let sender_callback = Arc::new(move |ev| {
            let mut c = sender.clone();
            wasm_bindgen_futures::spawn_local(async move {
                c.send(ev).await.unwrap();
            });
        });

        let mut nodes =
            HashMap::with_capacity_and_hasher(1000, nohash_hasher::BuildNoHashHasher::default());

        nodes.insert(
            0_u32,
            DomNode {
                node: root.clone().dyn_into::<Node>().unwrap(),
                meta: NodeMetadata::IsRoot,
            },
        );
        Self {
            stack: Stack::with_capacity(10),
            nodes,

            callback_id: 0,
            listeners: FxHashMap::default(),
            callback_map: FxHashMap::default(),
            document,
            event_receiver: receiver,
            trigger: sender_callback,
            root,
            last_node_was_text: false,
            building_style: false,
            node_counter: Counter(0),
        }
    }

    pub async fn wait_for_event(&mut self) -> Option<EventTrigger> {
        let v = self.event_receiver.recv().await.unwrap();
        Some(v)
    }
}

struct Counter(u32);
impl Counter {
    fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}
impl dioxus_core::diff::RealDom for WebsysDom {
    fn push_root(&mut self, root: dioxus_core::virtual_dom::RealDomNode) {
        log::debug!("Called `[`push_root] {:?}", root);
        let domnode = self.nodes.get(&root.0).expect("Failed to pop know root");
        self.stack.push(domnode.clone());
    }

    fn append_child(&mut self) {
        log::debug!("Called [`append_child`]");
        if self.building_style {
            self.building_style = false;
            return;
        }

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

        self.stack.top().append_child(&child).unwrap();
    }

    fn replace_with(&mut self) {
        log::debug!("Called [`replace_with`]");
        let new_node = self.stack.pop();
        let old_node = self.stack.pop();

        if old_node.has_type::<Element>() {
            old_node
                .dyn_ref::<Element>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else if old_node.has_type::<web_sys::CharacterData>() {
            old_node
                .dyn_ref::<web_sys::CharacterData>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else if old_node.has_type::<web_sys::DocumentType>() {
            old_node
                .dyn_ref::<web_sys::DocumentType>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else {
            panic!("Cannot replace node: {:?}", old_node);
        }

        // // poc to see if this is a valid solution
        // if let Some(id) = self.current_known {
        //     // update mapping
        //     self.known_roots.insert(id, new_node.clone());
        //     self.current_known = None;
        // }

        self.stack.push(new_node);
    }

    fn remove(&mut self) {
        log::debug!("Called [`remove`]");
        todo!()
    }

    fn remove_all_children(&mut self) {
        log::debug!("Called [`remove_all_children`]");
        todo!()
    }

    fn create_text_node(&mut self, text: &str) -> dioxus_core::virtual_dom::RealDomNode {
        let nid = self.node_counter.next();
        let textnode = self
            .document
            .create_text_node(text)
            .dyn_into::<Node>()
            .unwrap();

        let textnode = DomNode::new_text(textnode);

        self.stack.push(textnode.clone());
        self.nodes.insert(nid, textnode);

        log::debug!("Called [`create_text_node`]: {}, {}", text, nid);

        RealDomNode::new(nid)
    }

    fn create_element(&mut self, tag: &str) -> dioxus_core::virtual_dom::RealDomNode {
        let nid = self.node_counter.next();

        if tag == "style" {
            self.building_style = true;
            let real_node = self.stack.top();
            self.nodes.insert(nid, real_node.clone());
            // don't actually create any nodes
            // the following set attributes will actually apply to the node currently on the stack
        } else {
            let el = self
                .document
                .create_element(tag)
                .unwrap()
                .dyn_into::<Node>()
                .unwrap();

            let el = DomNode::new_nothing(el);

            self.stack.push(el.clone());
            self.nodes.insert(nid, el);
        }

        log::debug!("Called [`create_element`]: {}, {:?}", tag, nid);
        RealDomNode::new(nid)
    }

    fn create_element_ns(
        &mut self,
        tag: &str,
        namespace: &str,
    ) -> dioxus_core::virtual_dom::RealDomNode {
        let el = self
            .document
            .create_element_ns(Some(namespace), tag)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();
        let el = DomNode::new_nothing(el);

        self.stack.push(el.clone());
        let nid = self.node_counter.next();
        self.nodes.insert(nid, el);
        log::debug!("Called [`create_element_ns`]: {:}", nid);
        RealDomNode::new(nid)
    }

    fn new_event_listener(
        &mut self,
        event: &str,
        scope: dioxus_core::prelude::ScopeIdx,
        el_id: usize,
        real_id: RealDomNode,
    ) {
        log::debug!(
            "Called [`new_event_listener`]: {}, {:?}, {}, {:?}",
            event,
            scope,
            el_id,
            real_id
        );
        // attach the correct attributes to the element
        // these will be used by accessing the event's target
        // This ensures we only ever have one handler attached to the root, but decide
        // dynamically when we want to call a listener.

        let el = self.stack.top();

        let el = el
            .dyn_ref::<Element>()
            .expect(&format!("not an element: {:?}", el));

        let (gi_id, gi_gen) = (&scope).into_raw_parts();
        el.set_attribute(
            &format!("dioxus-event-{}", event),
            &format!("{}.{}.{}.{}", gi_id, gi_gen, el_id, real_id.0),
        )
        .unwrap();

        // Register the callback to decode

        if let Some(entry) = self.listeners.get_mut(event) {
            entry.0 += 1;
        } else {
            let trigger = self.trigger.clone();
            let handler = Closure::wrap(Box::new(move |event: &web_sys::Event| {
                // "Result" cannot be received from JS
                // Instead, we just build and immediately execute a closure that returns result
                let res = || -> anyhow::Result<EventTrigger> {
                    log::debug!("Handling event!");

                    let target = event
                        .target()
                        .expect("missing target")
                        .dyn_into::<Element>()
                        .expect("not a valid element");

                    let typ = event.type_();
                    use anyhow::Context;
                    let val: String = target
                        .get_attribute(&format!("dioxus-event-{}", typ))
                        .context("")?;

                    let mut fields = val.splitn(4, ".");

                    let gi_id = fields
                        .next()
                        .and_then(|f| f.parse::<usize>().ok())
                        .context("")?;
                    let gi_gen = fields
                        .next()
                        .and_then(|f| f.parse::<u64>().ok())
                        .context("")?;
                    let el_id = fields
                        .next()
                        .and_then(|f| f.parse::<usize>().ok())
                        .context("")?;
                    let real_id = fields
                        .next()
                        .and_then(|f| f.parse::<u32>().ok().map(RealDomNode::new))
                        .context("")?;

                    // Call the trigger
                    log::debug!(
                        "decoded gi_id: {},  gi_gen: {},  li_idx: {}",
                        gi_id,
                        gi_gen,
                        el_id
                    );

                    let triggered_scope = ScopeIdx::from_raw_parts(gi_id, gi_gen);
                    Ok(EventTrigger::new(
                        virtual_event_from_websys_event(event),
                        triggered_scope,
                        real_id,
                    ))
                };

                match res() {
                    Ok(synthetic_event) => trigger.as_ref()(synthetic_event),
                    Err(_) => log::error!("Error decoding Dioxus event attribute."),
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
        log::debug!("Called [`remove_event_listener`]: {}", event);
        todo!()
    }

    fn set_text(&mut self, text: &str) {
        log::debug!("Called [`set_text`]: {}", text);
        self.stack.top().set_text_content(Some(text))
    }

    fn set_attribute(&mut self, name: &str, value: &str, is_namespaced: bool) {
        if self.building_style {
            // if we're currently building a style
            if let Some(el) = self.stack.top().dyn_ref::<HtmlElement>() {
                let style_declaration = el.style();
                style_declaration.set_property(name, value);
            }
        }

        log::debug!("Called [`set_attribute`]: {}, {}", name, value);
        if name == "class" {
            if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                el.set_class_name(value);
            }
        } else {
            if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                el.set_attribute(name, value).unwrap();
            }
        }
    }

    fn remove_attribute(&mut self, name: &str) {
        log::debug!("Called [`remove_attribute`]: {}", name);
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

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        log::debug!("Called [`raw_node_as_any_mut`]");
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    list: Vec<DomNode>,
}

impl Stack {
    pub fn with_capacity(cap: usize) -> Self {
        Stack {
            list: Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, node: DomNode) {
        // pub fn push(&mut self, node: Node) {
        // debug!("stack-push: {:?}", node);
        self.list.push(node);
    }

    pub fn pop(&mut self) -> DomNode {
        // pub fn pop(&mut self) -> Node {
        let res = self.list.pop().unwrap();
        res
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn top(&self) -> &DomNode {
        // pub fn top(&self) -> &Node {
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
            impl dioxus_core::events::on::MouseEvent for CustomMouseEvent {
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
            VirtualEvent::MouseEvent(Rc::new(CustomMouseEvent(evt)))
            // MouseEvent(Box::new(RawMouseEvent {
            //                 alt_key: evt.alt_key(),
            //                 button: evt.button() as i32,
            //                 buttons: evt.buttons() as i32,
            //                 client_x: evt.client_x(),
            //                 client_y: evt.client_y(),
            //                 ctrl_key: evt.ctrl_key(),
            //                 meta_key: evt.meta_key(),
            //                 page_x: evt.page_x(),
            //                 page_y: evt.page_y(),
            //                 screen_x: evt.screen_x(),
            //                 screen_y: evt.screen_y(),
            //                 shift_key: evt.shift_key(),
            //                 get_modifier_state: GetModifierKey(Box::new(|f| {
            //                     // evt.get_modifier_state(f)
            //                     todo!("This is not yet implemented properly, sorry :(");
            //                 })),
            //             }))
            // todo!()
            // VirtualEvent::MouseEvent()
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
        _ => VirtualEvent::OtherEvent,
    }
}
