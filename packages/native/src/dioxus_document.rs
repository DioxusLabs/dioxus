//! Integration between Dioxus and Blitz

use std::{any::Any, collections::HashMap, rc::Rc, sync::Arc};

use blitz_dom::{
    local_name, namespace_url,
    net::Resource,
    node::{Attribute, NodeSpecificData},
    ns, Atom, BaseDocument, ElementNodeData, Node, NodeData, QualName, DEFAULT_CSS,
};

use blitz_traits::{net::NetProvider, ColorScheme, Document, DomEvent, DomEventData, Viewport};
use dioxus_core::{ElementId, Event, VirtualDom};
use dioxus_html::{set_event_converter, FormValue, PlatformEventData};
use futures_util::{pin_mut, FutureExt};

use super::event_handler::{BlitzKeyboardData, NativeClickData, NativeConverter, NativeFormData};
use crate::mutation_writer::{DioxusState, MutationWriter};
use crate::NodeId;

pub(crate) fn qual_name(local_name: &str, namespace: Option<&str>) -> QualName {
    QualName {
        prefix: None,
        ns: namespace.map(Atom::from).unwrap_or(ns!(html)),
        local: Atom::from(local_name),
    }
}

pub struct DioxusDocument {
    pub(crate) vdom: VirtualDom,
    pub(crate) vdom_state: DioxusState,
    pub(crate) inner: BaseDocument,

    #[allow(unused)]
    pub(crate) html_element_id: NodeId,
    #[allow(unused)]
    pub(crate) head_element_id: NodeId,
    #[allow(unused)]
    pub(crate) body_element_id: NodeId,
    #[allow(unused)]
    pub(crate) main_element_id: NodeId,
}

// Implement DocumentLike and required traits for DioxusDocument

impl AsRef<BaseDocument> for DioxusDocument {
    fn as_ref(&self) -> &BaseDocument {
        &self.inner
    }
}
impl AsMut<BaseDocument> for DioxusDocument {
    fn as_mut(&mut self) -> &mut BaseDocument {
        &mut self.inner
    }
}
impl From<DioxusDocument> for BaseDocument {
    fn from(doc: DioxusDocument) -> BaseDocument {
        doc.inner
    }
}
impl Document for DioxusDocument {
    type Doc = BaseDocument;

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

    fn id(&self) -> usize {
        self.inner.id()
    }

    fn handle_event(&mut self, event: &mut DomEvent) {
        let chain = self.inner.node_chain(event.target);

        set_event_converter(Box::new(NativeConverter {}));

        let renderer_event = event.clone();

        let mut prevent_default = false;
        let mut stop_propagation = false;

        match &event.data {
            DomEventData::MouseMove(data)
            | DomEventData::MouseDown(data)
            | DomEventData::MouseUp(data) => {
                let click_event_data = wrap_event_data(NativeClickData(data.clone()));

                for node_id in chain.clone().into_iter() {
                    let node = &self.inner.tree()[node_id];
                    let dioxus_id = node.element_data().and_then(DioxusDocument::dioxus_id);

                    if let Some(id) = dioxus_id {
                        let click_event = Event::new(click_event_data.clone(), true);
                        self.vdom
                            .runtime()
                            .handle_event(event.name(), click_event.clone(), id);
                        prevent_default |= !click_event.default_action_enabled();
                        stop_propagation |= !click_event.propagates();
                    }

                    if !event.bubbles || stop_propagation {
                        break;
                    }
                }
            }

            DomEventData::Click(data) => {
                let click_event_data = wrap_event_data(NativeClickData(data.clone()));

                for node_id in chain.clone().into_iter() {
                    let node = &self.inner.tree()[node_id];
                    let dioxus_id = node.element_data().and_then(DioxusDocument::dioxus_id);
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
                            let mut default_event =
                                DomEvent::new(node_id, renderer_event.data.clone());
                            self.inner.as_mut().handle_event(&mut default_event);
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
                                let DomEventData::Click(event) = &renderer_event.data else {
                                    unreachable!();
                                };
                                let input_click_data = self
                                    .inner
                                    .get_node(node_id)
                                    .unwrap()
                                    .synthetic_click_event(event.mods);
                                let mut default_event = DomEvent::new(node_id, input_click_data);
                                self.inner.as_mut().handle_event(&mut default_event);
                                prevent_default = true;

                                // TODO: Generated click events should bubble separately
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

                    if !event.bubbles || stop_propagation {
                        break;
                    }
                }
            }

            DomEventData::KeyPress(kevent) => {
                let key_event_data = wrap_event_data(BlitzKeyboardData(kevent.clone()));

                for node_id in chain.clone().into_iter() {
                    let node = &self.inner.tree()[node_id];
                    let dioxus_id = node.element_data().and_then(DioxusDocument::dioxus_id);
                    println!("{} {:?}", node_id, dioxus_id);

                    if let Some(id) = dioxus_id {
                        if kevent.state.is_pressed() {
                            // Handle keydown event
                            let event = Event::new(key_event_data.clone(), true);
                            self.vdom
                                .runtime()
                                .handle_event("keydown", event.clone(), id);
                            prevent_default |= !event.default_action_enabled();
                            stop_propagation |= !event.propagates();

                            if !prevent_default && kevent.text.is_some() {
                                // Handle keypress event
                                let event = Event::new(key_event_data.clone(), true);
                                self.vdom
                                    .runtime()
                                    .handle_event("keypress", event.clone(), id);
                                prevent_default |= !event.default_action_enabled();
                                stop_propagation |= !event.propagates();

                                if !prevent_default {
                                    // Handle default DOM event
                                    let mut default_event =
                                        DomEvent::new(node_id, renderer_event.data.clone());
                                    self.inner.as_mut().handle_event(&mut default_event);
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

                    if !event.bubbles || stop_propagation {
                        break;
                    }
                }
            }

            DomEventData::Hover => {}

            // TODO: Implement IME and Hover events handling
            DomEventData::Ime(_) => {}
        }

        if !event.cancelable || !prevent_default {
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
        parent_chain: &[usize],
        element_node_data: &ElementNodeData,
    ) -> NativeFormData {
        let parent_form = parent_chain.iter().find_map(|id| {
            let node = self.inner.get_node(*id)?;
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
        let viewport = Viewport::new(0, 0, 1.0, ColorScheme::Light);
        let mut doc = BaseDocument::new(viewport);

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
        root_node.children.push(html_element_id);

        // Create the body element
        let body_element_id = doc.create_node(NodeData::Element(ElementNodeData::new(
            qual_name("body", None),
            Vec::new(),
        )));
        let html_element = doc.get_node_mut(html_element_id).unwrap();
        html_element.children.push(body_element_id);
        let body_element = doc.get_node_mut(body_element_id).unwrap();
        body_element.parent = Some(html_element_id);

        // Create another virtual element to hold the root <div id="main"></div> under the html element
        let main_element_id = doc.create_node(NodeData::Element(ElementNodeData::new(
            qual_name("div", Some("id")),
            vec![blitz_dom::node::Attribute {
                name: qual_name("id", None),
                value: "main".to_string(),
            }],
        )));
        let body_element = doc.get_node_mut(body_element_id).unwrap();
        body_element.children.push(main_element_id);
        let main_element = doc.get_node_mut(main_element_id).unwrap();
        main_element.parent = Some(body_element_id);

        // Create the head element
        let head_element_id = doc.create_node(NodeData::Element(ElementNodeData::new(
            qual_name("head", None),
            Vec::new(),
        )));
        let head_element = doc.get_node_mut(head_element_id).unwrap();
        head_element.parent = Some(html_element_id);
        let html_element = doc.get_node_mut(html_element_id).unwrap();
        html_element.children.push(head_element_id);

        // Include default and user-specified stylesheets
        doc.add_user_agent_stylesheet(DEFAULT_CSS);

        let state = DioxusState::create(&mut doc, main_element_id);
        let mut doc = Self {
            vdom,
            vdom_state: state,
            inner: doc,
            html_element_id,
            head_element_id,
            body_element_id,
            main_element_id,
        };

        doc.inner.set_base_url("dioxus://index.html");

        // doc.initial_build();

        // doc.inner.print_tree();

        doc
    }

    pub fn initial_build(&mut self) {
        // let mut writer = MutationWriter::new(&mut self.inner, &mut self.vdom_state);

        // self.vdom.rebuild(&mut writer);
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

    pub fn create_head_element(
        &mut self,
        name: &str,
        attributes: &[(String, String)],
        contents: &Option<String>,
    ) {
        let mut stylesheet = None;
        let mut title = None;
        if name == "link" {
            let is_stylesheet = attributes
                .iter()
                .any(|(name, value)| name == "rel" && value == "stylesheet");
            if is_stylesheet {
                stylesheet = attributes
                    .iter()
                    .find(|(name, _value)| name == "href")
                    .map(|(_name, value)| value.clone());
            }
        }

        if name == "title" {
            title = attributes
                .iter()
                .find(|(name, _value)| name == "text")
                .and_then(|(_name, _value)| contents.clone());
        }

        let attributes = attributes
            .iter()
            .map(|(name, value)| Attribute {
                name: qual_name(name, None),
                value: value.clone(),
            })
            .collect();

        let new_element = self
            .inner
            .create_node(NodeData::Element(ElementNodeData::new(
                qual_name(name, None),
                attributes,
            )));

        if let Some(contents) = contents {
            let text_node = self.inner.create_text_node(contents);
            self.inner
                .get_node_mut(new_element)
                .unwrap()
                .children
                .push(text_node);
        }

        self.inner
            .get_node_mut(self.head_element_id)
            .unwrap()
            .children
            .push(new_element);

        if let Some(url) = stylesheet {
            crate::assets::fetch_linked_stylesheet(&self.inner, new_element, url);
        }

        if let Some(_title) = title {
            println!("todo: set title");
        }
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
