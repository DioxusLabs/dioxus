//! Implementation of a renderer for Dioxus on the web.
//!
//! Outstanding todos:
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?

use std::{any::Any, rc::Rc};

use dioxus_core::Runtime;
use dioxus_core::{ElementId, Template};
use dioxus_interpreter_js::unified_bindings::Interpreter;
use rustc_hash::FxHashMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Event, Node};

use crate::{load_document, virtual_event_from_websys_event, Config, WebEventConverter};

pub struct WebsysDom {
    #[allow(dead_code)]
    pub(crate) root: Node,
    pub(crate) document: Document,
    pub(crate) templates: FxHashMap<Template, u16>,
    pub(crate) interpreter: Interpreter,

    #[cfg(feature = "mounted")]
    pub(crate) runtime: Rc<Runtime>,

    #[cfg(feature = "mounted")]
    pub(crate) queued_mounted_events: Vec<ElementId>,

    // We originally started with a different `WriteMutations` for collecting templates during hydration.
    // When profiling the binary size of web applications, this caused a large increase in binary size
    // because diffing code in core is generic over the `WriteMutation` object.
    //
    // The fact that diffing is generic over WriteMutations instead of dynamic dispatch or a vec is nice
    // because we can directly write mutations to sledgehammer and avoid the runtime and binary size overhead
    // of dynamic dispatch
    //
    // Instead we now store a flag to see if we should be writing templates at all if hydration is enabled.
    // This has a small overhead, but it avoids dynamic dispatch and reduces the binary size
    //
    // NOTE: running the virtual dom with the `write_mutations` flag set to true is different from running
    // it with no mutation writer because it still assigns ids to nodes, but it doesn't write them to the dom
    #[cfg(feature = "hydrate")]
    pub(crate) skip_mutations: bool,

    #[cfg(feature = "hydrate")]
    pub(crate) suspense_hydration_ids: crate::hydration::SuspenseHydrationIds,
}

impl WebsysDom {
    pub fn new(cfg: Config, runtime: Rc<Runtime>) -> Self {
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
                (document, root.unchecked_into())
            }
            crate::cfg::ConfigRoot::RootNode(root) => {
                let document = match root.owner_document() {
                    Some(document) => document,
                    None => load_document(),
                };
                (document, root)
            }
        };

        let interpreter = Interpreter::default();

        // The closure type we pass to the dom may be invoked recursively if one event triggers another. For example,
        // one event could focus another element which triggers the focus event of the new element like inhttps://github.com/DioxusLabs/dioxus/issues/2882.
        // The Closure<dyn Fn(_)> type can invoked recursively, but Closure<dyn FnMut()> cannot
        let handler: Closure<dyn Fn(&Event)> = Closure::wrap(Box::new({
            let runtime = runtime.clone();
            move |web_sys_event: &web_sys::Event| {
                let name = web_sys_event.type_();
                let element = walk_event_for_id(web_sys_event);
                let bubbles = web_sys_event.bubbles();

                let Some((element, target)) = element else {
                    return;
                };

                let data = virtual_event_from_websys_event(web_sys_event.clone(), target);

                let event = dioxus_core::Event::new(Rc::new(data) as Rc<dyn Any>, bubbles);
                runtime.handle_event(name.as_str(), event.clone(), element);

                // Prevent the default action if the user set prevent default on the event
                let prevent_default = !event.default_action_enabled();
                // Prevent forms from submitting and redirecting
                if name == "submit" {
                    // On forms the default behavior is not to submit, if prevent default is set then we submit the form
                    if !prevent_default {
                        web_sys_event.prevent_default();
                    }
                } else if prevent_default {
                    web_sys_event.prevent_default();
                }
            }
        }));

        let _interpreter = interpreter.base();
        _interpreter.initialize(
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
            #[cfg(feature = "mounted")]
            runtime,
            #[cfg(feature = "mounted")]
            queued_mounted_events: Default::default(),
            #[cfg(feature = "hydrate")]
            skip_mutations: false,
            #[cfg(feature = "hydrate")]
            suspense_hydration_ids: Default::default(),
        }
    }
}

fn walk_event_for_id(event: &web_sys::Event) -> Option<(ElementId, web_sys::Element)> {
    let target = event
        .target()
        .expect("missing target")
        .dyn_into::<web_sys::Node>()
        .expect("not a valid node");

    walk_element_for_id(&target)
}

fn walk_element_for_id(target: &Node) -> Option<(ElementId, web_sys::Element)> {
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
