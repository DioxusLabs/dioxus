use std::{
    fmt::{Display, Formatter},
    rc::Rc,
};

use dioxus_core::{ElementId, Mutations, VirtualDom};
use dioxus_html::{
    geometry::euclid::{Point2D, Rect, Size2D},
    MountedData, MountedError, RenderedElementBacking,
};

use crate::query::{ElementRef, Query};

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

pub(crate) fn send_mounted_events(vdom: &mut VirtualDom, mount_events: Vec<ElementId>) {
    let query: Query = vdom
        .base_scope()
        .consume_context()
        .expect("Query should be in context");
    for id in mount_events {
        let element = TuiElement {
            query: query.clone(),
            id,
        };
        vdom.handle_event("mounted", Rc::new(MountedData::new(element)), id, false);
    }
}

struct TuiElement {
    query: Query,
    id: ElementId,
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

    fn get_raw_element(&self) -> dioxus_html::MountedResult<&dyn std::any::Any> {
        Ok(self)
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
