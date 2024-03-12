use std::{
    any::Any,
    fmt::{Display, Formatter},
};

use dioxus_core::{ElementId, WriteMutations};
use dioxus_html::{
    geometry::euclid::{Point2D, Rect, Size2D},
    MountedData, MountedError, RenderedElementBacking,
};

use dioxus_native_core::{dioxus::DioxusNativeCoreMutationWriter, NodeId};
use plasmo::query::{ElementRef, Query};

pub(crate) struct DioxusTUIMutationWriter<'a> {
    pub(crate) query: Query,
    pub(crate) events: &'a mut Vec<(ElementId, &'static str, Box<dyn Any>, bool)>,
    pub(crate) native_core_writer: DioxusNativeCoreMutationWriter<'a>,
}

impl WriteMutations for DioxusTUIMutationWriter<'_> {
    fn register_template(&mut self, template: dioxus_core::prelude::Template) {
        self.native_core_writer.register_template(template)
    }

    fn append_children(&mut self, id: ElementId, m: usize) {
        self.native_core_writer.append_children(id, m)
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.native_core_writer.assign_node_id(path, id)
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.native_core_writer.create_placeholder(id)
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.native_core_writer.create_text_node(value, id)
    }

    fn hydrate_text_node(&mut self, path: &'static [u8], value: &str, id: ElementId) {
        self.native_core_writer.hydrate_text_node(path, value, id)
    }

    fn load_template(&mut self, name: &'static str, index: usize, id: ElementId) {
        self.native_core_writer.load_template(name, index, id)
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.native_core_writer.replace_node_with(id, m)
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.native_core_writer
            .replace_placeholder_with_nodes(path, m)
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.native_core_writer.insert_nodes_after(id, m)
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.native_core_writer.insert_nodes_before(id, m)
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &dioxus_core::AttributeValue,
        id: ElementId,
    ) {
        self.native_core_writer.set_attribute(name, ns, value, id)
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.native_core_writer.set_node_text(value, id)
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if name == "mounted" {
            let element = TuiElement {
                query: self.query.clone(),
                id: self.native_core_writer.state.element_to_node_id(id),
            };
            self.events
                .push((id, "mounted", Box::new(MountedData::new(element)), false));
        } else {
            self.native_core_writer.create_event_listener(name, id)
        }
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.native_core_writer.remove_event_listener(name, id)
    }

    fn remove_node(&mut self, id: ElementId) {
        self.native_core_writer.remove_node(id)
    }

    fn push_root(&mut self, id: ElementId) {
        self.native_core_writer.push_root(id)
    }
}

#[derive(Clone)]
pub(crate) struct TuiElement {
    query: Query,
    id: NodeId,
}

impl TuiElement {
    pub(crate) fn element(&self) -> ElementRef {
        self.query.get(self.id)
    }
}

impl RenderedElementBacking for TuiElement {
    fn get_client_rect(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Future<
                Output = dioxus_html::MountedResult<dioxus_html::geometry::euclid::Rect<f64, f64>>,
            >,
        >,
    > {
        let layout = self.element().layout();
        Box::pin(async move {
            match layout {
                Some(layout) => {
                    let x = layout.location.x as f64;
                    let y = layout.location.y as f64;
                    let width = layout.size.width as f64;
                    let height = layout.size.height as f64;
                    Ok(Rect::new(Point2D::new(x, y), Size2D::new(width, height)))
                }
                None => Err(MountedError::OperationFailed(Box::new(TuiElementNotFound))),
            }
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct TuiElementNotFound;

impl Display for TuiElementNotFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TUI element not found")
    }
}

impl std::error::Error for TuiElementNotFound {}
