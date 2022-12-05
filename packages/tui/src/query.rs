use std::{
    cell::{Ref, RefCell},
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};

use dioxus_core::ElementId;
use taffy::{
    geometry::Point,
    prelude::{Layout, Size},
    Taffy,
};

use crate::TuiDom;

/// Allows querying the layout of nodes after rendering. It will only provide a correct value after a node is rendered.
/// Provided as a root context for all tui applictions.
/// # Example
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use dioxus_tui::query::Query;
/// use dioxus_tui::Size;
///
/// fn main() {
///     dioxus_tui::launch(app);
/// }
///
/// fn app(cx: Scope) -> Element {
///     let hue = use_state(&cx, || 0.0);
///     let brightness = use_state(&cx, || 0.0);
///     let tui_query: Query = cx.consume_context().unwrap();
///     cx.render(rsx! {
///         div{
///             width: "100%",
///             background_color: "hsl({hue}, 70%, {brightness}%)",
///             onmousemove: move |evt| {
///                 let node = tui_query.get(cx.root_node().mounted_id());
///                 let Size{width, height} = node.size().unwrap();
///                 hue.set((evt.data.offset_x as f32/width as f32)*255.0);
///                 brightness.set((evt.data.offset_y as f32/height as f32)*100.0);
///             },
///             "hsl({hue}, 70%, {brightness}%)",
///         }
///     })
/// }
/// ```
#[derive(Clone)]
pub struct Query {
    pub(crate) rdom: Rc<RefCell<TuiDom>>,
    pub(crate) stretch: Arc<Mutex<Taffy>>,
}

impl Query {
    pub fn get(&self, id: ElementId) -> ElementRef {
        ElementRef::new(
            self.rdom.borrow(),
            self.stretch.lock().expect("taffy lock poisoned"),
            id,
        )
    }
}

pub struct ElementRef<'a> {
    inner: Ref<'a, TuiDom>,
    stretch: MutexGuard<'a, Taffy>,
    id: ElementId,
}

impl<'a> ElementRef<'a> {
    fn new(inner: Ref<'a, TuiDom>, stretch: MutexGuard<'a, Taffy>, id: ElementId) -> Self {
        Self { inner, stretch, id }
    }

    pub fn size(&self) -> Option<Size<u32>> {
        self.layout().map(|l| l.size.map(|v| v as u32))
    }

    pub fn pos(&self) -> Option<Point<u32>> {
        self.layout().map(|l| Point {
            x: l.location.x as u32,
            y: l.location.y as u32,
        })
    }

    pub fn layout(&self) -> Option<&Layout> {
        self.stretch
            .layout(self.inner[self.id].state.layout.node.ok()?)
            .ok()
    }
}
