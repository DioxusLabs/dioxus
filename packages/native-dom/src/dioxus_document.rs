//! Integration between Dioxus and Blitz
use crate::events::{BlitzKeyboardData, NativeClickData, NativeConverter, NativeFormData};
use crate::mutation_writer::{DioxusState, MutationWriter};
use crate::qual_name;
use crate::NodeId;

use blitz_dom::Attribute;
use blitz_dom::{
    net::Resource, BaseDocument, Document, EventDriver, EventHandler, Node, DEFAULT_CSS,
};
use blitz_traits::{
    events::{DomEvent, DomEventData, EventState, UiEvent},
    net::NetProvider,
    shell::{ColorScheme, Viewport},
};

use dioxus_core::{ElementId, Event, VirtualDom};
use dioxus_html::{set_event_converter, PlatformEventData};
use futures_util::{pin_mut, FutureExt};
use std::ops::{Deref, DerefMut};
use std::{any::Any, collections::HashMap, rc::Rc, sync::Arc};

fn wrap_event_data<T: Any>(value: T) -> Rc<dyn Any> {
    Rc::new(PlatformEventData::new(Box::new(value)))
}

/// Get the value of the "dioxus-data-id" attribute parsed aa usize
fn get_dioxus_id(node: &Node) -> Option<ElementId> {
    node.element_data()?
        .attrs
        .iter()
        .find(|attr| *attr.name.local == *"data-dioxus-id")
        .and_then(|attr| attr.value.parse::<usize>().ok())
        .map(ElementId)
}

pub struct DioxusDocument {
    pub inner: BaseDocument,
    pub vdom: VirtualDom,
    pub vdom_state: DioxusState,

    #[allow(unused)]
    pub(crate) html_element_id: NodeId,
    #[allow(unused)]
    pub(crate) head_element_id: NodeId,
    #[allow(unused)]
    pub(crate) body_element_id: NodeId,
    #[allow(unused)]
    pub(crate) main_element_id: NodeId,
}

impl DioxusDocument {
    pub fn new(vdom: VirtualDom, net_provider: Option<Arc<dyn NetProvider<Resource>>>) -> Self {
        let viewport = Viewport::new(0, 0, 1.0, ColorScheme::Light);
        let mut doc = BaseDocument::new(viewport);

        // Set net provider
        if let Some(net_provider) = net_provider {
            doc.set_net_provider(net_provider);
        }

        // Create some minimal HTML to render the app into.

        // Create the html element
        let mut mutr = doc.mutate();
        let html_element_id = mutr.create_element(qual_name("html", None), vec![]);
        mutr.append_children(mutr.doc.root_node().id, &[html_element_id]);

        // Create the body element
        let head_element_id = mutr.create_element(qual_name("head", None), vec![]);
        mutr.append_children(html_element_id, &[head_element_id]);

        // Create the body element
        let body_element_id = mutr.create_element(qual_name("body", None), vec![]);
        mutr.append_children(html_element_id, &[body_element_id]);

        // Create another virtual element to hold the root <div id="main"></div> under the html element
        let main_attr = blitz_dom::Attribute {
            name: qual_name("id", None),
            value: "main".to_string(),
        };
        let main_element_id = mutr.create_element(qual_name("main", None), vec![main_attr]);
        mutr.append_children(body_element_id, &[main_element_id]);

        drop(mutr);

        // Include default and user-specified stylesheets
        doc.add_user_agent_stylesheet(DEFAULT_CSS);

        let vdom_state = DioxusState::create(main_element_id);
        let mut doc = Self {
            vdom,
            vdom_state,
            inner: doc,
            html_element_id,
            head_element_id,
            body_element_id,
            main_element_id,
        };

        doc.inner.set_base_url("dioxus://index.html");
        //doc.initial_build();
        doc.inner.print_tree();

        doc
    }

    pub fn initial_build(&mut self) {
        let mut writer = MutationWriter::new(&mut self.inner, &mut self.vdom_state);
        self.vdom.rebuild(&mut writer);
    }

    pub fn create_head_element(
        &mut self,
        name: &str,
        attributes: &[(String, String)],
        contents: &Option<String>,
    ) {
        let mut mutr = self.inner.mutate();

        let attributes = attributes
            .iter()
            .map(|(name, value)| Attribute {
                name: qual_name(name, None),
                value: value.clone(),
            })
            .collect();

        let new_elem_id = mutr.create_element(qual_name(name, None), attributes);
        mutr.append_children(self.head_element_id, &[new_elem_id]);
        if let Some(contents) = contents {
            mutr.append_text_to_node(new_elem_id, contents).unwrap();
        }

        // TODO: set title (we may also wish to have this built-in to the document?)
        // if name == "title" {
        //     let title = mutr.doc.nodes[new_elem_id].text_content();
        //
        // }
    }
}

// Implement DocumentLike and required traits for DioxusDocument
impl Document for DioxusDocument {
    fn id(&self) -> usize {
        self.inner.id()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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

    fn handle_event(&mut self, event: UiEvent) {
        set_event_converter(Box::new(NativeConverter {}));
        let handler = DioxusEventHandler {
            vdom: &mut self.vdom,
            vdom_state: &mut self.vdom_state,
        };
        let mut driver = EventDriver::new(self.inner.mutate(), handler);
        driver.handle_ui_event(event);
    }
}

impl Deref for DioxusDocument {
    type Target = BaseDocument;
    fn deref(&self) -> &BaseDocument {
        &self.inner
    }
}
impl DerefMut for DioxusDocument {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl From<DioxusDocument> for BaseDocument {
    fn from(doc: DioxusDocument) -> BaseDocument {
        doc.inner
    }
}

pub struct DioxusEventHandler<'v> {
    vdom: &'v mut VirtualDom,
    #[allow(dead_code, reason = "WIP")]
    vdom_state: &'v mut DioxusState,
}

impl EventHandler for DioxusEventHandler<'_> {
    fn handle_event(
        &mut self,
        chain: &[usize],
        event: &mut DomEvent,
        mutr: &mut blitz_dom::DocumentMutator<'_>,
        event_state: &mut EventState,
    ) {
        let event_data = match &event.data {
            DomEventData::MouseMove(mevent)
            | DomEventData::MouseDown(mevent)
            | DomEventData::MouseUp(mevent)
            | DomEventData::Click(mevent) => Some(wrap_event_data(NativeClickData(mevent.clone()))),

            DomEventData::KeyDown(kevent)
            | DomEventData::KeyUp(kevent)
            | DomEventData::KeyPress(kevent) => {
                Some(wrap_event_data(BlitzKeyboardData(kevent.clone())))
            }

            DomEventData::Input(data) => Some(wrap_event_data(NativeFormData {
                value: data.value.clone(),
                values: HashMap::new(),
            })),

            // TODO: Implement IME handling
            DomEventData::Ime(_) => None,
        };

        let Some(event_data) = event_data else {
            return;
        };

        for &node_id in chain {
            // Get dioxus vdom id for node
            let dioxus_id = mutr.doc.get_node(node_id).and_then(get_dioxus_id);
            let Some(id) = dioxus_id else {
                return;
            };

            // Handle event in vdom
            let dx_event = Event::new(event_data.clone(), event.bubbles);
            self.vdom
                .runtime()
                .handle_event(event.name(), dx_event.clone(), id);

            // Update event state
            if !dx_event.default_action_enabled() {
                event_state.prevent_default();
            }
            if !dx_event.propagates() {
                event_state.stop_propagation();
                break;
            }
        }
    }
}
