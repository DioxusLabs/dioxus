use std::{borrow::Borrow, convert::TryInto, default, fmt::Debug, sync::Arc};

use dioxus_core::{
    events::{EventTrigger, VirtualEvent},
    patch::Edit,
    prelude::ScopeIdx,
};
use fxhash::FxHashMap;
use log::debug;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{window, Document, Element, Event, HtmlInputElement, HtmlOptionElement, Node};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CacheId(u32);

#[derive(Clone)]
struct RootCallback(Arc<dyn Fn(EventTrigger)>);
impl std::fmt::Debug for RootCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
        // a no-op for now
        // todo!()
    }
}

#[derive(Debug)]
pub(crate) struct PatchMachine {
    pub(crate) stack: Stack,

    pub(crate) known_roots: FxHashMap<u32, Node>,

    pub(crate) root: Element,

    pub(crate) temporaries: FxHashMap<u32, Node>,

    pub(crate) document: Document,

    pub(crate) events: EventDelegater,

    pub(crate) current_known: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct EventDelegater {
    root: Element,

    // every callback gets a monotomically increasing callback ID
    callback_id: usize,

    // map of listener types to number of those listeners
    listeners: FxHashMap<String, (usize, Closure<dyn FnMut(&Event)>)>,

    // Map of callback_id to component index and listener id
    callback_map: FxHashMap<usize, (usize, usize)>,

    trigger: RootCallback,
}

impl EventDelegater {
    pub fn new(root: Element, trigger: impl Fn(EventTrigger) + 'static) -> Self {
        Self {
            trigger: RootCallback(Arc::new(trigger)),
            root,
            callback_id: 0,
            listeners: FxHashMap::default(),
            callback_map: FxHashMap::default(),
        }
    }

    pub fn add_listener(&mut self, event: &str, scope: ScopeIdx) {
        if let Some(entry) = self.listeners.get_mut(event) {
            entry.0 += 1;
        } else {
            let trigger = self.trigger.clone();
            let handler = Closure::wrap(Box::new(move |event: &web_sys::Event| {
                log::debug!("Handling event!");

                let target = event
                    .target()
                    .expect("missing target")
                    .dyn_into::<Element>()
                    .expect("not a valid element");

                let typ = event.type_();

                let gi_id: Option<usize> = target
                    .get_attribute(&format!("dioxus-giid-{}", typ))
                    .and_then(|v| v.parse().ok());

                let gi_gen: Option<u64> = target
                    .get_attribute(&format!("dioxus-gigen-{}", typ))
                    .and_then(|v| v.parse().ok());

                let li_idx: Option<usize> = target
                    .get_attribute(&format!("dioxus-lidx-{}", typ))
                    .and_then(|v| v.parse().ok());

                if let (Some(gi_id), Some(gi_gen), Some(li_idx)) = (gi_id, gi_gen, li_idx) {
                    // Call the trigger
                    log::debug!(
                        "decoded gi_id: {},  gi_gen: {},  li_idx: {}",
                        gi_id,
                        gi_gen,
                        li_idx
                    );

                    let triggered_scope = ScopeIdx::from_raw_parts(gi_id, gi_gen);
                    trigger.0.as_ref()(EventTrigger::new(
                        virtual_event_from_websys_event(event),
                        triggered_scope,
                        // scope,
                        li_idx,
                    ));
                }
            }) as Box<dyn FnMut(&Event)>);

            self.root
                .add_event_listener_with_callback(event, (&handler).as_ref().unchecked_ref())
                .unwrap();

            // Increment the listeners
            self.listeners.insert(event.into(), (1, handler));
        }
    }
}

// callback: RootCallback,
// callback: Option<Closure<dyn Fn(EventTrigger)>>,

#[derive(Debug, Default)]
pub struct Stack {
    list: Vec<Node>,
}

impl Stack {
    pub fn with_capacity(cap: usize) -> Self {
        Stack {
            list: Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, node: Node) {
        // debug!("stack-push: {:?}", node);
        self.list.push(node);
    }

    pub fn pop(&mut self) -> Node {
        let res = self.list.pop().unwrap();
        // debug!("stack-pop: {:?}", res);

        res
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn top(&self) -> &Node {
        // log::info!(
        //     "Called top of stack with {} items remaining",
        //     self.list.len()
        // );
        match self.list.last() {
            Some(a) => a,
            None => panic!("Called 'top' of an empty stack, make sure to push the root first"),
        }
    }
}

impl PatchMachine {
    pub fn new(root: Element, event_callback: impl Fn(EventTrigger) + 'static) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        // attach all listeners to the container element
        let events = EventDelegater::new(root.clone(), event_callback);

        Self {
            current_known: None,
            known_roots: Default::default(),
            root,
            events,
            stack: Stack::with_capacity(20),
            temporaries: Default::default(),
            document,
        }
    }

    pub fn unmount(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
        // self.templates.clear();
    }

    pub fn start(&mut self) {
        if let Some(child) = self.root.first_child() {
            self.stack.push(child);
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
    }

    pub fn get_template(&self, id: CacheId) -> Option<&Node> {
        todo!()
        // self.templates.get(&id)
    }

    pub fn handle_edit(&mut self, edit: &Edit) {
        match *edit {
            // 0
            Edit::SetText { text } => {
                //
                self.stack.top().set_text_content(Some(text))
            }

            // 1
            Edit::RemoveSelfAndNextSiblings {} => {
                let node = self.stack.pop();
                let mut sibling = node.next_sibling();

                while let Some(inner) = sibling {
                    let temp = inner.next_sibling();
                    if let Some(sibling) = inner.dyn_ref::<Element>() {
                        sibling.remove();
                    }
                    sibling = temp;
                }
                if let Some(node) = node.dyn_ref::<Element>() {
                    node.remove();
                }
            }

            // 2
            Edit::ReplaceWith => {
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

                // poc to see if this is a valid solution
                if let Some(id) = self.current_known {
                    // update mapping
                    self.known_roots.insert(id, new_node.clone());
                    self.current_known = None;
                }

                self.stack.push(new_node);
            }

            // 3
            Edit::SetAttribute { name, value } => {
                let node = self.stack.top();

                if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
                    node.set_attribute(name, value).unwrap();

                    // Some attributes are "volatile" and don't work through `setAttribute`.
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

            // 4
            Edit::RemoveAttribute { name } => {
                let node = self.stack.top();
                if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
                    node.remove_attribute(name).unwrap();

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

            // 5
            Edit::PushReverseChild { n } => {
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(children.length() - n - 1).unwrap();
                self.stack.push(child);
            }

            // 6
            Edit::PopPushChild { n } => {
                self.stack.pop();
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(n).unwrap();
                self.stack.push(child);
            }

            // 7
            Edit::Pop => {
                self.stack.pop();
            }

            // 8
            Edit::AppendChild => {
                let child = self.stack.pop();
                self.stack.top().append_child(&child).unwrap();
            }

            // 9
            Edit::CreateTextNode { text } => self.stack.push(
                self.document
                    .create_text_node(text)
                    .dyn_into::<Node>()
                    .unwrap(),
            ),

            // 10
            Edit::CreateElement { tag_name } => {
                let el = self
                    .document
                    .create_element(tag_name)
                    .unwrap()
                    .dyn_into::<Node>()
                    .unwrap();
                self.stack.push(el);
            }

            // 11
            Edit::NewListener { event, id, scope } => {
                // attach the correct attributes to the element
                // these will be used by accessing the event's target
                // This ensures we only ever have one handler attached to the root, but decide
                // dynamically when we want to call a listener.

                let el = self.stack.top();

                let el = el
                    .dyn_ref::<Element>()
                    .expect(&format!("not an element: {:?}", el));

                // el.add_event_listener_with_callback(
                //     event_type,
                //     self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
                // )
                // .unwrap();

                // debug!("adding attributes: {}, {}", a, b);

                // let CbIdx {
                //     gi_id,
                //     gi_gen,
                //     listener_idx: lidx,
                // } = idx;

                let (gi_id, gi_gen) = (&scope).into_raw_parts();
                el.set_attribute(&format!("dioxus-giid-{}", event), &gi_id.to_string())
                    .unwrap();
                el.set_attribute(&format!("dioxus-gigen-{}", event), &gi_gen.to_string())
                    .unwrap();
                el.set_attribute(&format!("dioxus-lidx-{}", event), &id.to_string())
                    .unwrap();

                self.events.add_listener(event, scope);
            }

            // 12
            Edit::UpdateListener { event, scope, id } => {
                // update our internal mapping, and then modify the attribute

                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    // el.set_attribute(&format!("dioxus-a-{}", event_type), &a.to_string())
                    //     .unwrap();
                    // el.set_attribute(&format!("dioxus-b-{}", event_type), &b.to_string())
                    //     .unwrap();
                }
            }

            // 13
            Edit::RemoveListener { event: event_type } => {
                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    // el.remove_event_listener_with_callback(
                    //     event_type,
                    //     self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
                    // )
                    // .unwrap();
                }
            }

            // 14
            Edit::CreateElementNs { tag_name, ns } => {
                let el = self
                    .document
                    .create_element_ns(Some(ns), tag_name)
                    .unwrap()
                    .dyn_into::<Node>()
                    .unwrap();
                self.stack.push(el);
            }

            // 15
            Edit::SaveChildrenToTemporaries {
                mut temp,
                start,
                end,
            } => {
                let parent = self.stack.top();
                let children = parent.child_nodes();
                for i in start..end {
                    self.temporaries.insert(temp, children.get(i).unwrap());
                    temp += 1;
                }
            }

            // 16
            Edit::PushChild { n } => {
                let parent = self.stack.top();
                // log::debug!("PushChild {:#?}", parent);
                let child = parent.child_nodes().get(n).unwrap();
                self.stack.push(child);
            }

            // 17
            Edit::PushTemporary { temp } => {
                let t = self.temporaries.get(&temp).unwrap().clone();
                self.stack.push(t);
            }

            // 18
            Edit::InsertBefore => {
                let before = self.stack.pop();
                let after = self.stack.pop();
                after
                    .parent_node()
                    .unwrap()
                    .insert_before(&before, Some(&after))
                    .unwrap();
                self.stack.push(before);
            }

            // 19
            Edit::PopPushReverseChild { n } => {
                self.stack.pop();
                let parent = self.stack.top();
                let children = parent.child_nodes();
                let child = children.get(children.length() - n - 1).unwrap();
                self.stack.push(child);
            }

            // 20
            Edit::RemoveChild { n } => {
                let parent = self.stack.top();
                if let Some(child) = parent.child_nodes().get(n).unwrap().dyn_ref::<Element>() {
                    child.remove();
                }
            }

            // 21
            Edit::SetClass { class_name } => {
                if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                    el.set_class_name(class_name);
                }
            }
            Edit::MakeKnown { node } => {
                let domnode = self.stack.top();
                self.known_roots.insert(node, domnode.clone());
            }
            Edit::TraverseToKnown { node } => {
                let domnode = self
                    .known_roots
                    .get(&node)
                    .expect("Failed to pop know root");
                self.current_known = Some(node);
                self.stack.push(domnode.clone());
            }
            Edit::RemoveKnown => {}
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

        "change" | "input" | "invalid" | "reset" | "submit" => {
            // is a special react events
            // let evt: web_sys::FormEvent = event.clone().dyn_into().unwrap();
            todo!()
        }

        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => {
            let evt: web_sys::MouseEvent = event.clone().dyn_into().unwrap();
            VirtualEvent::MouseEvent(MouseEvent(Box::new(RawMouseEvent {
                alt_key: evt.alt_key(),
                button: evt.button() as i32,
                buttons: evt.buttons() as i32,
                client_x: evt.client_x(),
                client_y: evt.client_y(),
                ctrl_key: evt.ctrl_key(),
                meta_key: evt.meta_key(),
                page_x: evt.page_x(),
                page_y: evt.page_y(),
                screen_x: evt.screen_x(),
                screen_y: evt.screen_y(),
                shift_key: evt.shift_key(),
                get_modifier_state: GetModifierKey(Box::new(|f| {
                    // evt.get_modifier_state(f)
                    todo!("This is not yet implemented properly, sorry :(");
                })),
            })))
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
