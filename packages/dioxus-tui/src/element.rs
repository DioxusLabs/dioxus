use std::{
    any::Any,
    fmt::{Display, Formatter},
};

use dioxus_core::{ElementId, Mutations, VirtualDom};
use dioxus_html::{
    geometry::euclid::{Point2D, Rect, Size2D},
    MountedData, MountedError, RenderedElementBacking,
};

use dioxus_native_core::NodeId;
use plasmo::query::{ElementRef, Query};

pub(crate) fn find_mount_events(mutations: &Mutations) -> Vec<ElementId> {
    let mut mount_events = Vec::new();
    for mutation in &mutations.edits {
        if let dioxus_core::Mutation::NewEventListener {
            name: "mounted",
            id,
        } = mutation
        {
            mount_events.push(*id);
        }
    }
    mount_events
}

// We need to queue the mounted events to give rink time to rendere and resolve the layout of elements after they are created
pub(crate) fn create_mounted_events(
    vdom: &VirtualDom,
    events: &mut Vec<(ElementId, &'static str, Box<dyn Any>, bool)>,
    mount_events: impl Iterator<Item = (ElementId, NodeId)>,
) {
    let query: Query = vdom
        .base_scope()
        .consume_context()
        .expect("Query should be in context");
    for (id, node_id) in mount_events {
        let element = TuiElement {
            query: query.clone(),
            id: node_id,
        };
        events.push((id, "mounted", Box::new(MountedData::new(element)), false));
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
