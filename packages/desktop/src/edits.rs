use dioxus_core::{BorrowedAttributeValue, Mutations, Template, TemplateAttribute, TemplateNode};
use dioxus_html::event_bubbles;
use dioxus_interpreter_js::binary_protocol::Channel;
use rustc_hash::FxHashMap;
use std::{
    sync::atomic::AtomicU16,
    sync::Arc,
    sync::{atomic::Ordering, Mutex},
};

use wry::RequestAsyncResponder;

/// This handles communication between the requests that the webview makes and the interpreter. The interpreter
/// constantly makes long running requests to the webview to get any edits that should be made to the DOM almost like
/// server side events.
///
/// It will hold onto the requests until the interpreter is ready to handle them and hold onto any pending edits until
/// a new request is made.
#[derive(Default, Clone)]
pub(crate) struct EditQueue {
    queue: Arc<Mutex<Vec<Vec<u8>>>>,
    responder: Arc<Mutex<Option<RequestAsyncResponder>>>,
}

impl EditQueue {
    pub fn handle_request(&self, responder: RequestAsyncResponder) {
        let mut queue = self.queue.lock().unwrap();
        if let Some(bytes) = queue.pop() {
            responder.respond(wry::http::Response::new(bytes));
        } else {
            *self.responder.lock().unwrap() = Some(responder);
        }
    }

    pub fn add_edits(&self, edits: Vec<u8>) {
        let mut responder = self.responder.lock().unwrap();
        if let Some(responder) = responder.take() {
            responder.respond(wry::http::Response::new(edits));
        } else {
            self.queue.lock().unwrap().push(edits);
        }
    }
}

pub(crate) fn apply_edits(
    mutations: Mutations,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &AtomicU16,
) -> Option<Vec<u8>> {
    if mutations.templates.is_empty() && mutations.edits.is_empty() {
        return None;
    }

    for template in mutations.templates {
        add_template(&template, channel, templates, max_template_count);
    }

    use dioxus_core::Mutation::*;
    for edit in mutations.edits {
        match edit {
            AppendChildren { id, m } => channel.append_children(id.0 as u32, m as u16),
            AssignId { path, id } => channel.assign_id(path, id.0 as u32),
            CreatePlaceholder { id } => channel.create_placeholder(id.0 as u32),
            CreateTextNode { value, id } => channel.create_text_node(value, id.0 as u32),
            HydrateText { path, value, id } => channel.hydrate_text(path, value, id.0 as u32),
            LoadTemplate { name, index, id } => {
                if let Some(tmpl_id) = templates.get(name) {
                    channel.load_template(*tmpl_id, index as u16, id.0 as u32)
                }
            }
            ReplaceWith { id, m } => channel.replace_with(id.0 as u32, m as u16),
            ReplacePlaceholder { path, m } => channel.replace_placeholder(path, m as u16),
            InsertAfter { id, m } => channel.insert_after(id.0 as u32, m as u16),
            InsertBefore { id, m } => channel.insert_before(id.0 as u32, m as u16),
            SetAttribute {
                name,
                value,
                id,
                ns,
            } => match value {
                BorrowedAttributeValue::Text(txt) => {
                    channel.set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Float(f) => {
                    channel.set_attribute(id.0 as u32, name, &f.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Int(n) => {
                    channel.set_attribute(id.0 as u32, name, &n.to_string(), ns.unwrap_or_default())
                }
                BorrowedAttributeValue::Bool(b) => channel.set_attribute(
                    id.0 as u32,
                    name,
                    if b { "true" } else { "false" },
                    ns.unwrap_or_default(),
                ),
                BorrowedAttributeValue::None => {
                    channel.remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
                }
                _ => unreachable!(),
            },
            SetText { value, id } => channel.set_text(id.0 as u32, value),
            NewEventListener { name, id, .. } => {
                channel.new_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            RemoveEventListener { name, id } => {
                channel.remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8)
            }
            Remove { id } => channel.remove(id.0 as u32),
            PushRoot { id } => channel.push_root(id.0 as u32),
        }
    }

    let bytes: Vec<_> = channel.export_memory().collect();
    channel.reset();
    Some(bytes)
}

pub fn add_template(
    template: &Template<'static>,
    channel: &mut Channel,
    templates: &mut FxHashMap<String, u16>,
    max_template_count: &AtomicU16,
) {
    let current_max_template_count = max_template_count.load(Ordering::Relaxed);
    for root in template.roots.iter() {
        create_template_node(channel, root);
        templates.insert(template.name.to_owned(), current_max_template_count);
    }
    channel.add_templates(current_max_template_count, template.roots.len() as u16);

    max_template_count.fetch_add(1, Ordering::Relaxed);
}

pub fn create_template_node(channel: &mut Channel, node: &'static TemplateNode<'static>) {
    use TemplateNode::*;
    match node {
        Element {
            tag,
            namespace,
            attrs,
            children,
            ..
        } => {
            // Push the current node onto the stack
            match namespace {
                Some(ns) => channel.create_element_ns(tag, ns),
                None => channel.create_element(tag),
            }
            // Set attributes on the current node
            for attr in *attrs {
                if let TemplateAttribute::Static {
                    name,
                    value,
                    namespace,
                } = attr
                {
                    channel.set_top_attribute(name, value, namespace.unwrap_or_default())
                }
            }
            // Add each child to the stack
            for child in *children {
                create_template_node(channel, child);
            }
            // Add all children to the parent
            channel.append_children_to_top(children.len() as u16);
        }
        Text { text } => channel.create_raw_text(text),
        DynamicText { .. } => channel.create_raw_text("p"),
        Dynamic { .. } => channel.add_placeholder(),
    }
}
