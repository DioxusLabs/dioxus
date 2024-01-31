//! Implementation of a renderer for Dioxus on the web.
//!
//! Outstanding todos:
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?

use dioxus_core::ElementId;
use dioxus_html::PlatformEventData;
use dioxus_interpreter_js::Channel;
use futures_channel::mpsc;
use rustc_hash::FxHashMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event};

use crate::{load_document, virtual_event_from_websys_event, Config, WebEventConverter};

pub struct WebsysDom {
    pub(crate) document: Document,
    #[allow(dead_code)]
    pub(crate) root: Element,
    pub(crate) templates: FxHashMap<String, u16>,
    pub(crate) max_template_id: u16,
    pub(crate) interpreter: Channel,
    #[cfg(feature = "mounted")]
    pub(crate) event_channel: mpsc::UnboundedSender<UiEvent>,
    #[cfg(feature = "mounted")]
    pub(crate) queued_mounted_events: Vec<ElementId>,
}

pub struct UiEvent {
    pub name: String,
    pub bubbles: bool,
    pub element: ElementId,
    pub data: PlatformEventData,
}

//fn get_document(elem: &web_sys::Element) ->

impl WebsysDom {
    pub fn new(cfg: Config, event_channel: mpsc::UnboundedSender<UiEvent>) -> Self {
        let (document, root) = match cfg.root {
            crate::cfg::ConfigRoot::RootName(rootname) => {
                // eventually, we just want to let the interpreter do all the work of decoding events into our event type
                // a match here in order to avoid some error during runtime browser test
                let document = load_document();
                let root = match document.get_element_by_id(&rootname) {
                    Some(root) => root,
                    None => {
                        web_sys::console::error_1(
                            &format!("element '#{}' not found. mounting to the body.", rootname)
                                .into(),
                        );
                        document.create_element("body").ok().unwrap()
                    }
                };
                (document, root)
            }
            crate::cfg::ConfigRoot::RootElement(root) => {
                let document = match root.owner_document() {
                    Some(document) => document,
                    None => load_document(),
                };
                (document, root)
            }
        };

        let interpreter = Channel::default();

        let handler: Closure<dyn FnMut(&Event)> = Closure::wrap(Box::new({
            let event_channel = event_channel.clone();
            move |event: &web_sys::Event| {
                let name = event.type_();
                let element = walk_event_for_id(event);
                let bubbles = dioxus_html::event_bubbles(name.as_str());
                if let Some((element, target)) = element {
                    let prevent_event;
                    if let Some(prevent_requests) = target
                        .get_attribute("dioxus-prevent-default")
                        .as_deref()
                        .map(|f| f.split_whitespace())
                    {
                        prevent_event = prevent_requests
                            .map(|f| f.trim_start_matches("on"))
                            .any(|f| f == name);
                    } else {
                        prevent_event = false;
                    }

                    // Prevent forms from submitting and redirecting
                    if name == "submit" {
                        // On forms the default behavior is not to submit, if prevent default is set then we submit the form
                        if !prevent_event {
                            event.prevent_default();
                        }
                    } else if prevent_event {
                        event.prevent_default();
                    }

                    let data = virtual_event_from_websys_event(event.clone(), target);
                    let _ = event_channel.unbounded_send(UiEvent {
                        name,
                        bubbles,
                        element,
                        data,
                    });
                }
            }
        }));

        dioxus_interpreter_js::initialize(
            root.clone().unchecked_into(),
            handler.as_ref().unchecked_ref(),
        );
        dioxus_html::set_event_converter(Box::new(WebEventConverter));
        handler.forget();
        Self {
            document,
            root,
            interpreter,
            templates: FxHashMap::default(),
            max_template_id: 0,
            #[cfg(feature = "mounted")]
            event_channel,
            #[cfg(feature = "mounted")]
            queued_mounted_events: Default::default(),
        }
    }

    pub fn mount(&mut self) {
        self.interpreter.mount_to_root();
    }
}

fn walk_event_for_id(event: &web_sys::Event) -> Option<(ElementId, web_sys::Element)> {
    let target = event
        .target()
        .expect("missing target")
        .dyn_into::<web_sys::Node>()
        .expect("not a valid node");
    let mut current_target_element = target.dyn_ref::<web_sys::Element>().cloned();

    loop {
        match (
            current_target_element
                .as_ref()
                .and_then(|el| el.get_attribute("data-dioxus-id").map(|f| f.parse())),
            current_target_element,
        ) {
            // This node is an element, and has a dioxus id, so we can stop walking
            (Some(Ok(id)), Some(target)) => return Some((ElementId(id), target)),

            // Walk the tree upwards until we actually find an event target
            (None, target_element) => {
                let parent = match target_element.as_ref() {
                    Some(el) => el.parent_element(),
                    // if this is the first node and not an element, we need to get the parent from the target node
                    None => target.parent_element(),
                };
                match parent {
                    Some(parent) => current_target_element = Some(parent),
                    _ => return None,
                }
            }

            // This node is an element with an invalid dioxus id, give up
            _ => return None,
        }
    }
}
