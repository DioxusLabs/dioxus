//! Integration between Dioxus and Blitz

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use blitz_dom::{
    events::{EventData, RendererEvent},
    local_name, namespace_url,
    net::Resource,
    node::{Attribute, NodeSpecificData},
    ns, Atom, Document, DocumentLike, ElementNodeData, Node, NodeData, QualName, Viewport,
    DEFAULT_CSS,
};

use blitz_traits::net::NetProvider;
use dioxus::{
    dioxus_core::{
        AttributeValue, ElementId, Event, Template, TemplateAttribute, TemplateNode, VirtualDom,
        WriteMutations,
    },
    html::{FormValue, PlatformEventData},
    prelude::set_event_converter,
};
use futures_util::{pin_mut, FutureExt};
use rustc_hash::FxHashMap;
use style::{
    data::{ElementData, ElementStyles},
    invalidation::element::restyle_hints::RestyleHint,
    properties::{style_structs::Font, ComputedValues},
};

use crate::documents::keyboard_event::BlitzKeyboardData;

use super::event_handler::{NativeClickData, NativeConverter, NativeFormData};

type NodeId = usize;

fn qual_name(local_name: &str, namespace: Option<&str>) -> QualName {
    QualName {
        prefix: None,
        ns: namespace.map(Atom::from).unwrap_or(ns!(html)),
        local: Atom::from(local_name),
    }
}

pub struct DioxusDocument {
    pub(crate) vdom: VirtualDom,
    vdom_state: DioxusState,
    inner: Document,
}

// Implement DocumentLike and required traits for DioxusDocument

pub struct DxNodeIds {
    node_id: usize,
    dioxus_id: Option<ElementId>,
}

impl AsRef<Document> for DioxusDocument {
    fn as_ref(&self) -> &Document {
        &self.inner
    }
}
impl AsMut<Document> for DioxusDocument {
    fn as_mut(&mut self) -> &mut Document {
        &mut self.inner
    }
}
impl From<DioxusDocument> for Document {
    fn from(doc: DioxusDocument) -> Document {
        doc.inner
    }
}
impl DocumentLike for DioxusDocument {
    fn poll(&mut self, mut cx: std::task::Context) -> bool {
        {
            let fut = self.vdom.wait_for_work();
            pin_mut!(fut);

            match fut.poll_unpin(&mut cx) {
                std::task::Poll::Ready(_) => {}
                std::task::Poll::Pending => return false,
            }
        }

        let mut writer = MutationWriter::new(&mut self.inner, &mut self.vdom_state);
        self.vdom.render_immediate(&mut writer);

        true
    }

    fn handle_event(&mut self, event: blitz_dom::events::RendererEvent) {
        // Collect the nodes into a chain by traversing upwards
        // This is important so the "capture" phase can be implemented
        let mut next_node_id = Some(event.target);
        let mut chain = Vec::with_capacity(16);

        // if it's a capturing event, we want to fill in the chain with the parent nodes
        // until we reach the root - that way we can call the listeners in the correct order
        // otherwise, we just want to call the listeners on the target
        //
        // todo: this is harcoded for "click" events - eventually we actually need to handle proper propagation
        // if event.name == "click" {
        while let Some(node_id) = next_node_id {
            let node = &self.inner.tree()[node_id];

            if let Some(element) = node.element_data() {
                let dioxus_id = DioxusDocument::dioxus_id(element);
                chain.push(DxNodeIds { node_id, dioxus_id })
            }

            next_node_id = node.parent;
        }

        set_event_converter(Box::new(NativeConverter {}));

        let renderer_event = event.clone();

        let mut prevent_default = false;
        let mut stop_propagation = false;

        match &event.data {
            EventData::Click { .. } => {
                // look for the data-dioxus-id attribute on the element
                // todo: we might need to walk upwards to find the first element with a data-dioxus-id attribute

                let click_event_data = wrap_event_data(NativeClickData);

                for &DxNodeIds { node_id, dioxus_id } in chain.iter() {
                    let mut trigger_label = false;

                    if let Some(id) = dioxus_id {
                        // Trigger click event
                        let click_event = Event::new(click_event_data.clone(), true);
                        self.vdom
                            .runtime()
                            .handle_event("click", click_event.clone(), id);
                        prevent_default |= !click_event.default_action_enabled();
                        stop_propagation |= !click_event.propagates();

                        if !prevent_default {
                            let default_event = RendererEvent {
                                target: node_id,
                                data: renderer_event.data.clone(),
                            };
                            self.inner.as_mut().handle_event(default_event);
                            prevent_default = true;

                            let element = self.inner.tree()[node_id].element_data().unwrap();
                            trigger_label = element.name.local == *"label";

                            //TODO Check for other inputs which trigger input event on click here, eg radio
                            let triggers_input_event = element.name.local == local_name!("input")
                                && element.attr(local_name!("type")) == Some("checkbox");
                            if triggers_input_event {
                                let form_data =
                                    wrap_event_data(self.input_event_form_data(&chain, element));
                                let input_event = Event::new(form_data, true);
                                self.vdom.runtime().handle_event("input", input_event, id);
                            }
                        }
                    }

                    //Clicking labels triggers click, and possibly input event, of bound input
                    if trigger_label {
                        if let Some((dioxus_id, node_id)) = self.label_bound_input_element(node_id)
                        {
                            let click_event = Event::new(click_event_data.clone(), true);
                            self.vdom.runtime().handle_event(
                                "click",
                                click_event.clone(),
                                dioxus_id,
                            );

                            // Handle default DOM event
                            if click_event.default_action_enabled() {
                                let &EventData::Click { mods, .. } = &renderer_event.data else {
                                    unreachable!();
                                };
                                let input_click_data = self
                                    .inner
                                    .get_node(node_id)
                                    .unwrap()
                                    .synthetic_click_event(mods);
                                let default_event = RendererEvent {
                                    target: node_id,
                                    data: input_click_data,
                                };
                                self.inner.as_mut().handle_event(default_event);
                                prevent_default = true;

                                // TODO: Generated click events should bubble separatedly
                                // prevent_default |= !click_event.default_action_enabled();

                                //TODO Check for other inputs which trigger input event on click here, eg radio
                                let element_data = self
                                    .inner
                                    .get_node(node_id)
                                    .unwrap()
                                    .element_data()
                                    .unwrap();
                                let triggers_input_event =
                                    element_data.attr(local_name!("type")) == Some("checkbox");
                                let form_data = wrap_event_data(
                                    self.input_event_form_data(&chain, element_data),
                                );
                                if triggers_input_event {
                                    let input_event = Event::new(form_data, true);
                                    self.vdom.runtime().handle_event(
                                        "input",
                                        input_event,
                                        dioxus_id,
                                    );
                                }
                            }
                        }
                    }

                    if stop_propagation {
                        break;
                    }
                }
            }
            EventData::KeyPress {
                event: wevent,
                mods,
            } => {
                let key_event_data =
                    wrap_event_data(BlitzKeyboardData::from_winit(wevent, mods.state()));

                for &DxNodeIds { node_id, dioxus_id } in chain.iter() {
                    println!("{} {:?}", node_id, dioxus_id);

                    if let Some(id) = dioxus_id {
                        if wevent.state.is_pressed() {
                            // Handle keydown event
                            let event = Event::new(key_event_data.clone(), true);
                            self.vdom
                                .runtime()
                                .handle_event("keydown", event.clone(), id);
                            prevent_default |= !event.default_action_enabled();
                            stop_propagation |= !event.propagates();

                            if !prevent_default && wevent.text.is_some() {
                                // Handle keypress event
                                let event = Event::new(key_event_data.clone(), true);
                                self.vdom
                                    .runtime()
                                    .handle_event("keypress", event.clone(), id);
                                prevent_default |= !event.default_action_enabled();
                                stop_propagation |= !event.propagates();

                                if !prevent_default {
                                    // Handle default DOM event
                                    let default_event = RendererEvent {
                                        target: node_id,
                                        data: renderer_event.data.clone(),
                                    };
                                    self.inner.as_mut().handle_event(default_event);
                                    prevent_default = true;

                                    // Handle input event
                                    let element =
                                        self.inner.tree()[node_id].element_data().unwrap();
                                    let triggers_input_event = &element.name.local == "input"
                                        && matches!(
                                            element.attr(local_name!("type")),
                                            None | Some("text" | "password" | "email" | "search")
                                        );
                                    if triggers_input_event {
                                        let form_data = wrap_event_data(dbg!(
                                            self.input_event_form_data(&chain, element)
                                        ));
                                        let input_event = Event::new(form_data, true);
                                        self.vdom.runtime().handle_event("input", input_event, id);
                                    }
                                }
                            }
                        } else {
                            // Handle keyup event
                            let event = Event::new(key_event_data.clone(), true);
                            self.vdom.runtime().handle_event("keyup", event.clone(), id);
                            prevent_default |= !event.default_action_enabled();
                            stop_propagation |= !event.propagates();
                        }
                    }

                    if stop_propagation {
                        break;
                    }
                }
            }
            EventData::Ime(_) => todo!(),
            EventData::Hover => todo!(),
        }

        if !prevent_default {
            self.inner.as_mut().handle_event(event);
        }
    }
}

fn wrap_event_data<T: Any>(value: T) -> Rc<dyn Any> {
    Rc::new(PlatformEventData::new(Box::new(value)))
}

impl DioxusDocument {
    /// Generate the FormData from an input event
    /// Currently only cares about input checkboxes
    pub fn input_event_form_data(
        &self,
        parent_chain: &[DxNodeIds],
        element_node_data: &ElementNodeData,
    ) -> NativeFormData {
        let parent_form = parent_chain.iter().map(|ids| ids.node_id).find_map(|id| {
            let node = self.inner.get_node(id)?;
            let element_data = node.element_data()?;
            if element_data.name.local == local_name!("form") {
                Some(node)
            } else {
                None
            }
        });
        let values = if let Some(parent_form) = parent_form {
            let mut values = HashMap::<String, FormValue>::new();
            for form_input in self.input_descendents(parent_form).into_iter() {
                // Match html behaviour here. To be included in values:
                // - input must have a name
                // - if its an input, we only include it if checked
                // - if value is not specified, it defaults to 'on'
                if let Some(name) = form_input.attr(local_name!("name")) {
                    if form_input.attr(local_name!("type")) == Some("checkbox")
                        && form_input
                            .element_data()
                            .and_then(|data| data.checkbox_input_checked())
                            .unwrap_or(false)
                    {
                        let value = form_input
                            .attr(local_name!("value"))
                            .unwrap_or("on")
                            .to_string();
                        values.insert(name.to_string(), FormValue(vec![value]));
                    }
                }
            }
            values
        } else {
            Default::default()
        };
        let value = match &element_node_data.node_specific_data {
            NodeSpecificData::CheckboxInput(checked) => checked.to_string(),
            NodeSpecificData::TextInput(input_data) => input_data.editor.text().to_string(),
            _ => element_node_data
                .attr(local_name!("value"))
                .unwrap_or_default()
                .to_string(),
        };

        NativeFormData { value, values }
    }

    /// Collect all the inputs which are descendents of a given node
    fn input_descendents(&self, node: &Node) -> Vec<&Node> {
        node.children
            .iter()
            .flat_map(|id| {
                let mut res = Vec::<&Node>::new();
                let Some(n) = self.inner.get_node(*id) else {
                    return res;
                };
                let Some(element_data) = n.element_data() else {
                    return res;
                };
                if element_data.name.local == local_name!("input") {
                    res.push(n);
                }
                res.extend(self.input_descendents(n).iter());
                res
            })
            .collect()
    }

    pub fn new(
        vdom: VirtualDom,
        net_provider: Option<Arc<dyn NetProvider<Data = Resource>>>,
    ) -> Self {
        let viewport = Viewport::new(0, 0, 1.0);
        let mut doc = Document::new(viewport);

        // Set net provider
        if let Some(net_provider) = net_provider {
            doc.set_net_provider(net_provider);
        }

        // Create a virtual "html" element to act as the root element, as we won't necessarily
        // have a single root otherwise, while the rest of blitz requires that we do
        let html_element_id = doc.create_node(NodeData::Element(ElementNodeData::new(
            qual_name("html", None),
            Vec::new(),
        )));
        let root_node_id = doc.root_node().id;
        let html_element = doc.get_node_mut(html_element_id).unwrap();
        html_element.parent = Some(root_node_id);
        let root_node = doc.get_node_mut(root_node_id).unwrap();
        // Stylo data on the root node container is needed to render the element
        let stylo_element_data = ElementData {
            styles: ElementStyles {
                primary: Some(
                    ComputedValues::initial_values_with_font_override(Font::initial_values())
                        .to_arc(),
                ),
                ..Default::default()
            },
            ..Default::default()
        };
        *root_node.stylo_element_data.borrow_mut() = Some(stylo_element_data);
        root_node.children.push(html_element_id);

        // Include default and user-specified stylesheets
        doc.add_user_agent_stylesheet(DEFAULT_CSS);

        let state = DioxusState::create(&mut doc);
        let mut doc = Self {
            vdom,
            vdom_state: state,
            inner: doc,
        };

        doc.initial_build();

        doc.inner.print_tree();

        doc
    }

    pub fn initial_build(&mut self) {
        let mut writer = MutationWriter::new(&mut self.inner, &mut self.vdom_state);
        self.vdom.rebuild(&mut writer);
        // dbg!(self.vdom.rebuild_to_vec());
        // std::process::exit(0);
        // dbg!(writer.state);
    }

    pub fn label_bound_input_element(&self, label_node_id: NodeId) -> Option<(ElementId, NodeId)> {
        let bound_input_elements = self.inner.label_bound_input_elements(label_node_id);

        // Filter down bound elements to those which have dioxus id
        bound_input_elements.into_iter().find_map(|n| {
            let target_element_data = n.element_data()?;
            let node_id = n.id;
            let dioxus_id = DioxusDocument::dioxus_id(target_element_data)?;
            Some((dioxus_id, node_id))
        })
    }

    fn dioxus_id(element_node_data: &ElementNodeData) -> Option<ElementId> {
        Some(ElementId(
            element_node_data
                .attrs
                .iter()
                .find(|attr| *attr.name.local == *"data-dioxus-id")?
                .value
                .parse::<usize>()
                .ok()?,
        ))
    }

    // pub fn apply_mutations(&mut self) {
    //     // Apply the mutations to the actual dom
    //     let mut writer = MutationWriter {
    //         doc: &mut self.inner,
    //         state: &mut self.vdom_state,
    //     };
    //     self.vdom.render_immediate(&mut writer);

    //     println!("APPLY MUTATIONS");
    //     self.inner.print_tree();
    // }
}

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Store of templates keyed by unique name
    templates: FxHashMap<Template, Vec<NodeId>>,
    /// Stack machine state for applying dioxus mutations
    stack: Vec<NodeId>,
    /// Mapping from vdom ElementId -> rdom NodeId
    node_id_mapping: Vec<Option<NodeId>>,
}

/// A writer for mutations that can be used with the RealDom.
pub struct MutationWriter<'a> {
    /// The realdom associated with this writer
    pub doc: &'a mut Document,

    /// The state associated with this writer
    pub state: &'a mut DioxusState,

    pub style_nodes: HashSet<usize>,
}

impl<'a> MutationWriter<'a> {
    fn new(doc: &'a mut Document, state: &'a mut DioxusState) -> Self {
        MutationWriter {
            doc,
            state,
            style_nodes: HashSet::new(),
        }
    }

    fn is_style_node(&self, node_id: NodeId) -> bool {
        self.doc
            .get_node(node_id)
            .unwrap()
            .raw_dom_data
            .is_element_with_tag_name(&local_name!("style"))
    }

    fn maybe_push_style_node(&mut self, node_id: impl Into<Option<NodeId>>) {
        if let Some(node_id) = node_id.into() {
            if self.is_style_node(node_id) {
                self.style_nodes.insert(node_id);
            }
        }
    }

    #[track_caller]
    fn maybe_push_parent_style_node(&mut self, node_id: NodeId) {
        let parent_id = self.doc.get_node(node_id).unwrap().parent;
        self.maybe_push_style_node(parent_id);
    }
}

impl Drop for MutationWriter<'_> {
    fn drop(&mut self) {
        // Add/Update inline stylesheets (<style> elements)
        for &id in &self.style_nodes {
            self.doc.upsert_stylesheet_for_node(id);
        }
    }
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create(doc: &mut Document) -> Self {
        let root = doc.root_element();
        let root_id = root.id;

        Self {
            templates: FxHashMap::default(),
            stack: vec![root_id],
            node_id_mapping: vec![Some(root_id)],
        }
    }

    /// Convert an ElementId to a NodeId
    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.try_element_to_node_id(element_id).unwrap()
    }

    /// Attempt to convert an ElementId to a NodeId. This will return None if the ElementId is not in the RealDom.
    pub fn try_element_to_node_id(&self, element_id: ElementId) -> Option<NodeId> {
        self.node_id_mapping.get(element_id.0).copied().flatten()
    }

    // /// Create a mutation writer for the RealDom
    // pub fn create_mutation_writer<'a>(&'a mut self, doc: &'a mut Document) -> MutationWriter<'a> {
    //     MutationWriter { doc, state: self }
    // }
}

impl MutationWriter<'_> {
    /// Update an ElementId -> NodeId mapping
    fn set_id_mapping(&mut self, node_id: NodeId, element_id: ElementId) {
        let element_id: usize = element_id.0;

        // Ensure node_id_mapping is large enough to contain element_id
        if self.state.node_id_mapping.len() <= element_id {
            self.state.node_id_mapping.resize(element_id + 1, None);
        }

        // Set the new mapping
        self.state.node_id_mapping[element_id] = Some(node_id);
    }

    /// Find a child in the document by child index path
    fn load_child(&self, path: &[u8]) -> NodeId {
        let mut current = self
            .doc
            .get_node(*self.state.stack.last().unwrap())
            .unwrap();
        for i in path {
            let new_id = current.children[*i as usize];
            current = self.doc.get_node(new_id).unwrap();
        }
        current.id
    }
}

impl WriteMutations for MutationWriter<'_> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("append_children id:{} m:{}", id.0, m);

        let children = self.state.stack.split_off(self.state.stack.len() - m);
        let parent = self.state.element_to_node_id(id);
        for child in children {
            self.doc.get_node_mut(parent).unwrap().children.push(child);
            self.doc.get_node_mut(child).unwrap().parent = Some(parent);
        }

        self.maybe_push_style_node(parent);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("assign_node_id path:{:?} id:{}", path, id.0);

        // If there is an existing node already mapped to that ID and
        // it has no parent, then drop it
        if let Some(node_id) = self.state.try_element_to_node_id(id) {
            if let Some(node) = self.doc.get_node(node_id) {
                if node.parent.is_none() {
                    self.doc.remove_and_drop_node(node_id);
                }
            }
        }

        let node_id = self.load_child(path);
        self.set_id_mapping(node_id, id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("create_placeholder id:{}", id.0);

        let node_id = self.doc.create_node(NodeData::Comment);
        self.set_id_mapping(node_id, id);
        self.state.stack.push(node_id);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("create_text_node id:{} text:{}", id.0, value);

        let node_id = self.doc.create_text_node(value);
        self.set_id_mapping(node_id, id);
        self.state.stack.push(node_id);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let template_entry = self.state.templates.entry(template).or_insert_with(|| {
            let template_root_ids: Vec<NodeId> = template
                .roots
                .iter()
                .map(|root| create_template_node(self.doc, root))
                .collect();

            template_root_ids
        });

        let template_node_id = template_entry[index];
        let clone_id = self.doc.deep_clone_node(template_node_id);
        self.set_id_mapping(clone_id, id);
        self.state.stack.push(clone_id);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("replace_node_with id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        self.maybe_push_parent_style_node(anchor_node_id);
        self.doc.insert_before(anchor_node_id, &new_nodes);
        self.doc.remove_node(anchor_node_id);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("replace_placeholder_with_nodes path:{:?} m:{}", path, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.load_child(path);
        self.maybe_push_parent_style_node(anchor_node_id);
        self.doc.insert_before(anchor_node_id, &new_nodes);
        self.doc.remove_node(anchor_node_id);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("insert_nodes_after id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        let next_sibling_id = self
            .doc
            .get_node(anchor_node_id)
            .unwrap()
            .forward(1)
            .map(|node| node.id);

        match next_sibling_id {
            Some(anchor_node_id) => {
                self.doc.insert_before(anchor_node_id, &new_nodes);
            }
            None => self.doc.append(anchor_node_id, &new_nodes),
        }

        self.maybe_push_parent_style_node(anchor_node_id);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("insert_nodes_before id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        self.doc.insert_before(anchor_node_id, &new_nodes);

        self.maybe_push_parent_style_node(anchor_node_id);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let node_id = self.state.element_to_node_id(id);

        #[cfg(feature = "tracing")]
        tracing::info!(
            "set_attribute node_id:{} ns: {:?} name:{}, value:{:?}",
            node_id,
            ns,
            name,
            value
        );

        self.doc.snapshot_node(node_id);

        let node = &mut self.doc.nodes[node_id];

        let stylo_element_data = &mut *node.stylo_element_data.borrow_mut();
        if let Some(data) = stylo_element_data {
            data.hint |= RestyleHint::restyle_subtree();
        }

        if let NodeData::Element(ref mut element) = node.raw_dom_data {
            if element.name.local == local_name!("input") && name == "checked" {
                set_input_checked_state(element, value);
            }
            // FIXME: support other non-text attributes
            else if let AttributeValue::Text(val) = value {
                // Update text input value
                if let Some(input_data) = element.text_input_data_mut() {
                    input_data.set_text(&mut self.doc.font_ctx, &mut self.doc.layout_ctx, val);
                }

                // FIXME check namespace
                let existing_attr = element
                    .attrs
                    .iter_mut()
                    .find(|attr| attr.name.local == *name);

                if let Some(existing_attr) = existing_attr {
                    existing_attr.value.clear();
                    existing_attr.value.push_str(val);
                } else {
                    // we have overloaded the style namespace to accumulate style attributes without a `style` block
                    if ns == Some("style") {
                        // todo: need to accumulate style attributes into a single style
                        //
                        // element.
                    } else {
                        element.attrs.push(Attribute {
                            name: qual_name(name, ns),
                            value: val.to_string(),
                        });
                    }
                }
            }

            if let AttributeValue::None = value {
                // Update text input value
                if let Some(input_data) = element.text_input_data_mut() {
                    input_data.set_text(&mut self.doc.font_ctx, &mut self.doc.layout_ctx, "");
                }

                // FIXME: check namespace
                element.attrs.retain(|attr| attr.name.local != *name);
            }
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("set_node_text id:{} value:{}", id.0, value);

        let node_id = self.state.element_to_node_id(id);
        let node = self.doc.get_node_mut(node_id).unwrap();

        let text = match node.raw_dom_data {
            NodeData::Text(ref mut text) => text,
            // todo: otherwise this is basically element.textContent which is a bit different - need to parse as html
            _ => return,
        };

        let changed = text.content != value;
        if changed {
            text.content.clear();
            text.content.push_str(value);
            let parent = node.parent;
            self.maybe_push_style_node(parent);
        }
    }

    fn create_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // we're going to actually set the listener here as a placeholder - in JS this would also be a placeholder
        // we might actually just want to attach the attribute to the root element (delegation)
        self.set_attribute(
            _name,
            None,
            &AttributeValue::Text("<rust func>".into()),
            _id,
        );

        // also set the data-dioxus-id attribute so we can find the element later
        self.set_attribute(
            "data-dioxus-id",
            None,
            &AttributeValue::Text(_id.0.to_string()),
            _id,
        );

        // let node_id = self.state.element_to_node_id(id);
        // let mut node = self.rdom.get_mut(node_id).unwrap();
        // node.add_event_listener(name);
    }

    fn remove_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // let node_id = self.state.element_to_node_id(id);
        // let mut node = self.rdom.get_mut(node_id).unwrap();
        // node.remove_event_listener(name);
    }

    fn remove_node(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("remove_node id:{}", id.0);

        let node_id = self.state.element_to_node_id(id);
        self.doc.remove_node(node_id);
    }

    fn push_root(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("push_root id:{}", id.0,);

        let node_id = self.state.element_to_node_id(id);
        self.state.stack.push(node_id);
    }
}

/// Set 'checked' state on an input based on given attributevalue
fn set_input_checked_state(element: &mut ElementNodeData, value: &AttributeValue) {
    let checked: bool;
    match value {
        AttributeValue::Bool(checked_bool) => {
            checked = *checked_bool;
        }
        AttributeValue::Text(val) => {
            if let Ok(checked_bool) = val.parse() {
                checked = checked_bool;
            } else {
                return;
            };
        }
        _ => {
            return;
        }
    };
    match element.node_specific_data {
        NodeSpecificData::CheckboxInput(ref mut checked_mut) => *checked_mut = checked,
        // If we have just constructed the element, set the node attribute,
        // and NodeSpecificData will be created from that later
        // this simulates the checked attribute being set in html,
        // and the element's checked property being set from that
        NodeSpecificData::None => element.attrs.push(Attribute {
            name: QualName {
                prefix: None,
                ns: ns!(html),
                local: local_name!("checked"),
            },
            value: checked.to_string(),
        }),
        _ => {}
    }
}

fn create_template_node(doc: &mut Document, node: &TemplateNode) -> NodeId {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let name = qual_name(tag, *namespace);
            let attrs = attrs
                .iter()
                .filter_map(|attr| match attr {
                    TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } => Some(Attribute {
                        name: qual_name(name, *namespace),
                        value: value.to_string(),
                    }),
                    TemplateAttribute::Dynamic { .. } => None,
                })
                .collect();

            let mut data = ElementNodeData::new(name, attrs);
            data.flush_style_attribute(doc.guard());

            let id = doc.create_node(NodeData::Element(data));
            let node = doc.get_node(id).unwrap();

            // Initialise style data
            *node.stylo_element_data.borrow_mut() = Some(Default::default());

            // If the node has an "id" attribute, store it in the ID map.
            // FIXME: implement
            // if let Some(id_attr) = node.attr(local_name!("id")) {
            //     doc.nodes_to_id.insert(id_attr.to_string(), id);
            // }

            let child_ids: Vec<NodeId> = children
                .iter()
                .map(|child| create_template_node(doc, child))
                .collect();
            for &child_id in &child_ids {
                doc.get_node_mut(child_id).unwrap().parent = Some(id);
            }
            doc.get_node_mut(id).unwrap().children = child_ids;

            id
        }
        TemplateNode::Text { text } => doc.create_text_node(text),
        TemplateNode::Dynamic { .. } => doc.create_node(NodeData::Comment),
    }
}
