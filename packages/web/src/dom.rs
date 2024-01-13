//! Implementation of a renderer for Dioxus on the web.
//!
//! Outstanding todos:
//! - Passive event listeners
//! - no-op event listener patch for safari
//! - tests to ensure dyn_into works for various event types.
//! - Partial delegation?

use dioxus_core::{
    BorrowedAttributeValue, ElementId, Mutation, Template, TemplateAttribute, TemplateNode,
};
use dioxus_html::event_bubbles;
use dioxus_html::PlatformEventData;
use dioxus_interpreter_js::{get_node, minimal_bindings, save_template, Channel};
use futures_channel::mpsc;
use rustc_hash::FxHashMap;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Document, Element, Event};

use crate::{load_document, virtual_event_from_websys_event, Config, WebEventConverter};

pub struct WebsysDom {
    document: Document,
    #[allow(dead_code)]
    pub(crate) root: Element,
    templates: FxHashMap<String, u16>,
    max_template_id: u16,
    pub(crate) interpreter: Channel,
    #[cfg(feature = "mounted")]
    event_channel: mpsc::UnboundedSender<UiEvent>,
}

pub struct UiEvent {
    pub name: String,
    pub bubbles: bool,
    pub element: ElementId,
    pub data: PlatformEventData,
}

impl WebsysDom {
    pub fn new(cfg: Config, event_channel: mpsc::UnboundedSender<UiEvent>) -> Self {
        // eventually, we just want to let the interpreter do all the work of decoding events into our event type
        // a match here in order to avoid some error during runtime browser test
        let document = load_document();
        let root = match document.get_element_by_id(&cfg.rootname) {
            Some(root) => root,
            None => {
                web_sys::console::error_1(
                    &format!(
                        "element '#{}' not found. mounting to the body.",
                        cfg.rootname
                    )
                    .into(),
                );
                document.create_element("body").ok().unwrap()
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
                        minimal_bindings::setAttributeInner(
                            el.clone().into(),
                            name,
                            JsValue::from_str(value),
                            *namespace,
                        );
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
        #[cfg(feature = "mounted")]
        // we need to apply the mount events last, so we collect them here
        let mut to_mount = Vec::new();
        for edit in &edits {
            match edit {
                AppendChildren { id, m } => i.append_children(id.0 as u32, *m as u16),
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
                        i.load_template(*tmpl_id, *index as u16, id.0 as u32)
                    }
                }
                ReplaceWith { id, m } => i.replace_with(id.0 as u32, *m as u16),
                ReplacePlaceholder { path, m } => {
                    i.replace_placeholder(path.as_ptr() as u32, path.len() as u8, *m as u16)
                }
                InsertAfter { id, m } => i.insert_after(id.0 as u32, *m as u16),
                InsertBefore { id, m } => i.insert_before(id.0 as u32, *m as u16),
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
                    match *name {
                        // mounted events are fired immediately after the element is mounted.
                        "mounted" => {
                            #[cfg(feature = "mounted")]
                            to_mount.push(*id);
                        }
                        _ => {
                            i.new_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
                        }
                    }
                }
                RemoveEventListener { name, id } => match *name {
                    "mounted" => {}
                    _ => {
                        i.remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
                    }
                },
                Remove { id } => i.remove(id.0 as u32),
                PushRoot { id } => i.push_root(id.0 as u32),
            }
        }
        edits.clear();
        i.flush();

        #[cfg(feature = "mounted")]
        for id in to_mount {
            self.send_mount_event(id);
        }
    }

    pub(crate) fn send_mount_event(&self, id: ElementId) {
        let node = get_node(id.0 as u32);
        if let Some(element) = node.dyn_ref::<Element>() {
            let _ = self.event_channel.unbounded_send(UiEvent {
                name: "mounted".to_string(),
                bubbles: false,
                element: id,
                data: PlatformEventData::new(Box::new(element.clone())),
            });
        }
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
